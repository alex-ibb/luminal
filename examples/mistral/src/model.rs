use std::{marker::PhantomData, ops::Div};

use luminal::{
    nn::{embedding::Embedding, norm::RMSNorm},
    prelude::*,
    shape::symbolic::{BigExpression, Expression},
};

// Mistral 7B Config
pub const VOCAB_SIZE: usize = 32000;
pub const HIDDEN_DIM: usize = 4096;
pub const NUM_LAYERS: usize = 32;
pub const N_HEADS: usize = 32;
pub const N_KV_HEADS: usize = 8;
pub const MLP_DIM: usize = 14336;

pub const N_ATTENTION_GROUPS: usize = N_HEADS / N_KV_HEADS;
pub const HEAD_DIM: usize = HIDDEN_DIM / N_HEADS;
pub const HEAD_DIM_OVER_2: usize = HEAD_DIM / 2;
pub const ATTN_PROJ_DIM: usize = HEAD_DIM * N_KV_HEADS;

pub type KVCache<Batch, Seq> = (
    GraphTensor<(Batch, Const<N_KV_HEADS>, Seq, Const<HEAD_DIM>)>,
    GraphTensor<(Batch, Const<N_KV_HEADS>, Seq, Const<HEAD_DIM>)>,
);

pub struct Mlp<const I: usize, const H: usize> {
    pub gate_proj: GraphTensor<(Const<I>, Const<H>)>,
    pub down_proj: GraphTensor<(Const<H>, Const<I>)>,
    pub up_proj: GraphTensor<(Const<I>, Const<H>)>,
}

impl<Sh: Shape, Im: Shape, const I: usize, const H: usize> Module<GraphTensor<Sh>> for Mlp<I, H>
where
    GraphTensor<Sh>: Matmul<R2<H, I>, Output = GraphTensor<Im>>,
    GraphTensor<Im>: Matmul<R2<I, H>, Output = GraphTensor<Sh>>,
{
    type Output = GraphTensor<Sh>;

    fn forward(&self, input: GraphTensor<Sh>) -> Self::Output {
        let gate = input.matmul(self.gate_proj.permute()).swish();
        let up = input.matmul(self.up_proj.permute()) * gate;
        up.matmul(self.down_proj.permute())
    }
}

impl<const I: usize, const H: usize> InitModule for Mlp<I, H> {
    fn initialize(cx: &mut Graph) -> Self {
        Self {
            gate_proj: cx.named_tensor("Gate Weight"),
            up_proj: cx.named_tensor("Up Weight"),
            down_proj: cx.named_tensor("Down Weight"),
        }
    }
}

impl<const I: usize, const H: usize> SerializeModule for Mlp<I, H> {
    fn serialize(&self, s: &mut Serializer) {
        s.tensor("ffn_gate/weight", self.gate_proj);
        s.tensor("ffn_up/weight", self.up_proj);
        s.tensor("ffn_down/weight", self.down_proj);
    }
}

fn apply_rotary_embeddings_ggml<const N_HEADS: usize, Batch: Dimension, Seq: Dimension>(
    input: GraphTensor<(Batch, Const<N_HEADS>, Seq, Const<HEAD_DIM>)>,
    prev_seq: BigExpression,
) -> GraphTensor<(Batch, Const<N_HEADS>, Seq, Const<HEAD_DIM>)> {
    // Get freqs
    let freqs = (input.graph().arange::<Const<HEAD_DIM_OVER_2>>() * 2.0) / (HEAD_DIM as f32);
    let freqs = freqs.inv_pow(1000000.0).recip();
    let pos = input.graph().arange::<Seq>() + prev_seq;
    let emb = pos.expand::<(_, Const<1>), _>().matmul(freqs.expand());

    // Split input into evens and odds
    let split = input.reshape::<(Batch, Const<N_HEADS>, Seq, Const<HEAD_DIM_OVER_2>, Const<2>)>();
    let x0: GraphTensor<(Batch, Const<N_HEADS>, Seq, Const<HEAD_DIM_OVER_2>, Const<1>)> = split
        .slice((.., .., .., .., ..Expression::from(1)))
        .contiguous()
        .realize();
    let x1: GraphTensor<(Batch, Const<N_HEADS>, Seq, Const<HEAD_DIM_OVER_2>, Const<1>)> = split
        .slice((.., .., .., .., Expression::from(1)..))
        .contiguous()
        .realize();

    // Apply sin and cos embeddings
    let x0_out = x0 * emb.cos().expand() - x1 * emb.sin().expand();
    let x1_out = x0 * emb.sin().expand() + x1 * emb.cos().expand();

    // Combine back into output
    x0_out
        .concat_along::<(Batch, Const<N_HEADS>, Seq, Const<HEAD_DIM_OVER_2>, Const<2>), Axis<4>, _>(
            x1_out,
        )
        .reshape()
}

pub struct SelfAttention {
    pub q_proj: GraphTensor<R2<HIDDEN_DIM, HIDDEN_DIM>>,
    pub k_proj: GraphTensor<R2<ATTN_PROJ_DIM, HIDDEN_DIM>>,
    pub v_proj: GraphTensor<R2<ATTN_PROJ_DIM, HIDDEN_DIM>>,
    pub o_proj: GraphTensor<R2<HIDDEN_DIM, HIDDEN_DIM>>,
}

impl<Batch: Dimension, CurSeq: Dimension, PrevSeq: Dimension, TotSeq: Dimension>
    Module<(
        GraphTensor<(Batch, CurSeq, Const<HIDDEN_DIM>)>,
        Option<KVCache<Batch, PrevSeq>>,
        PhantomData<TotSeq>,
    )> for SelfAttention
{
    type Output = (
        GraphTensor<(Batch, CurSeq, Const<HIDDEN_DIM>)>,
        KVCache<Batch, TotSeq>,
    );
    fn forward(
        &self,
        (x, cache, _): (
            GraphTensor<(Batch, CurSeq, Const<HIDDEN_DIM>)>,
            Option<KVCache<Batch, PrevSeq>>,
            PhantomData<TotSeq>,
        ),
    ) -> Self::Output {
        // Apply the Projections
        let queries = x
            .matmul(self.q_proj.permute())
            .reshape::<(Batch, CurSeq, Const<N_HEADS>, Const<HEAD_DIM>)>()
            .permute::<_, Axes4<0, 2, 1, 3>>();
        let keys = x
            .matmul(self.k_proj.permute())
            .reshape::<(Batch, CurSeq, Const<N_KV_HEADS>, Const<HEAD_DIM>)>()
            .permute::<_, Axes4<0, 2, 1, 3>>();
        let values = x
            .matmul(self.v_proj.permute())
            .reshape::<(Batch, CurSeq, Const<N_KV_HEADS>, Const<HEAD_DIM>)>()
            .permute::<_, Axes4<0, 2, 1, 3>>();

        // Rotary embed queries and keys
        let queries = apply_rotary_embeddings_ggml(queries, PrevSeq::const_size().into());
        let keys = apply_rotary_embeddings_ggml(keys, PrevSeq::const_size().into());

        // Add KV cache
        let (keys, values) = if let Some((k_cache, v_cache)) = cache {
            (
                k_cache.concat_along::<_, Axis<2>, _>(keys),
                v_cache.concat_along::<_, Axis<2>, _>(values),
            )
        } else {
            (keys.realize(), values.contiguous().realize())
        };

        // Repeat the KV States for Grouped-Query Attention
        let repeated_keys = keys.expand::<(_, _, Const<N_ATTENTION_GROUPS>, _, _), _>();
        let repeated_values = values.expand::<(_, _, Const<N_ATTENTION_GROUPS>, _, _), _>();

        // Calculate attention weights
        let mut attention_weights = queries
            .reshape::<(_, Const<N_KV_HEADS>, Const<N_ATTENTION_GROUPS>, _, _)>() // Split query heads into groups
            .matmul(repeated_keys.permute())
            .div((HEAD_DIM as f32).sqrt());

        let attention_mask = self.k_proj.graph().triu::<CurSeq>(1) * f16::MIN.to_f32();
        attention_weights += attention_mask
            .pad::<(CurSeq, TotSeq), _, _>(&[
                (0.into(), Expression::from(0)),
                (TotSeq::const_size() - CurSeq::const_size(), 0.into()),
            ])
            .expand();

        // Calculate final outputs
        let output = attention_weights
            .softmax::<4>()
            // Apply distribution to values
            .matmul(repeated_values)
            // Merge heads
            .permute::<_, Axes5<0, 3, 1, 2, 4>>()
            .reshape::<(Batch, CurSeq, Const<HIDDEN_DIM>)>();
        let output = output
            // Apply output projection
            .matmul(self.o_proj.permute());
        (output, (keys.contiguous(), values.contiguous())) // Cache needs to be contiguous for transferring to another graph
    }
}

impl InitModule for SelfAttention {
    fn initialize(cx: &mut Graph) -> Self {
        Self {
            q_proj: cx.named_tensor("Q Proj"),
            k_proj: cx.named_tensor("K Proj"),
            v_proj: cx.named_tensor("V Proj"),
            o_proj: cx.named_tensor("O Proj"),
        }
    }
}

impl SerializeModule for SelfAttention {
    fn serialize(&self, s: &mut Serializer) {
        s.tensor("attn_q/weight", self.q_proj);
        s.tensor("attn_v/weight", self.v_proj);
        s.tensor("attn_k/weight", self.k_proj);
        s.tensor("attn_output/weight", self.o_proj);
    }
}

pub struct TransformerBlock {
    pub attention: SelfAttention,
    pub attention_norm: RMSNorm<HIDDEN_DIM>,
    pub feed_forward: Mlp<MLP_DIM, HIDDEN_DIM>,
    pub feed_forward_norm: RMSNorm<HIDDEN_DIM>,
}

impl<Batch: Dimension, CurSeq: Dimension, PrevSeq: Dimension, TotSeq: Dimension>
    Module<(
        GraphTensor<(Batch, CurSeq, Const<HIDDEN_DIM>)>,
        Option<KVCache<Batch, PrevSeq>>,
        PhantomData<TotSeq>,
    )> for TransformerBlock
{
    type Output = (
        GraphTensor<(Batch, CurSeq, Const<HIDDEN_DIM>)>,
        KVCache<Batch, TotSeq>,
    );
    fn forward(
        &self,
        (mut x, cache, _): (
            GraphTensor<(Batch, CurSeq, Const<HIDDEN_DIM>)>,
            Option<KVCache<Batch, PrevSeq>>,
            PhantomData<TotSeq>,
        ),
    ) -> Self::Output {
        // Attention
        let normed = self.attention_norm.forward(x);
        let (y, cache) = self
            .attention
            .forward((normed, cache, PhantomData::<TotSeq>));

        // Residual Addition
        x += y;

        // Feed Forward
        let y = self.feed_forward.forward(self.feed_forward_norm.forward(x));

        // Residual Addition
        (x + y, cache)
    }
}

impl InitModule for TransformerBlock {
    fn initialize(cx: &mut Graph) -> Self {
        Self {
            attention: InitModule::initialize(cx),
            attention_norm: {
                let mut norm = RMSNorm::initialize(cx);
                norm.epsilon = 1e-5;
                norm
            },
            feed_forward: InitModule::initialize(cx),
            feed_forward_norm: {
                let mut norm = RMSNorm::initialize(cx);
                norm.epsilon = 1e-5;
                norm
            },
        }
    }
}

impl SerializeModule for TransformerBlock {
    fn serialize(&self, s: &mut Serializer) {
        s.module("", &self.attention);
        s.module("attn_norm", &self.attention_norm);
        s.module("ffn_norm", &self.feed_forward_norm);
        s.module("", &self.feed_forward);
    }
}

pub struct MistralLM {
    // Token embeddings
    pub embedding: Embedding<VOCAB_SIZE, HIDDEN_DIM>,
    // Transformer layers
    pub layers: Vec<TransformerBlock>,
    // Final Norm layer
    pub norm: RMSNorm<HIDDEN_DIM>,
    // LM Head Layer
    pub lm_head: GraphTensor<R2<VOCAB_SIZE, HIDDEN_DIM>>,
}

impl<Batch: Dimension, CurSeq: Dimension, PrevSeq: Dimension, TotSeq: Dimension>
    Module<(
        GraphTensor<(Batch, CurSeq)>,
        Option<Vec<KVCache<Batch, PrevSeq>>>,
        PhantomData<TotSeq>,
    )> for MistralLM
{
    type Output = (
        GraphTensor<(Batch, CurSeq, Const<VOCAB_SIZE>)>,
        Vec<KVCache<Batch, TotSeq>>,
    );
    fn forward(
        &self,
        (input, cache, _): (
            GraphTensor<(Batch, CurSeq)>,
            Option<Vec<KVCache<Batch, PrevSeq>>>,
            PhantomData<TotSeq>,
        ),
    ) -> Self::Output {
        // Embed tokens
        let mut x = self.embedding.forward(input);

        // Run through layers and collect new caches
        let mut new_caches = vec![];
        let mut new_cache;
        for (i, layer) in self.layers.iter().enumerate() {
            (x, new_cache) =
                layer.forward((x, cache.as_ref().map(|c| c[i]), PhantomData::<TotSeq>));
            new_caches.push(new_cache);
        }
        // Run through last norm and output projection
        let output = self.norm.forward(x).matmul(self.lm_head.permute());

        (output, new_caches)
    }
}

impl InitModule for MistralLM {
    fn initialize(cx: &mut Graph) -> Self {
        Self {
            embedding: InitModule::initialize(cx),
            norm: {
                let mut norm = RMSNorm::initialize(cx);
                norm.epsilon = 1e-5;
                norm
            },
            lm_head: cx.named_tensor("LM Head"),
            layers: (0..NUM_LAYERS)
                .map(|_| InitModule::initialize(cx))
                .collect(),
        }
    }
}

impl SerializeModule for MistralLM {
    fn serialize(&self, s: &mut Serializer) {
        s.module("token_embd", &self.embedding);
        s.module("output_norm", &self.norm);
        s.tensor("output/weight", self.lm_head);
        for (i, layer) in self.layers.iter().enumerate() {
            s.module(&format!("blk/{i}"), layer);
        }
    }
}
