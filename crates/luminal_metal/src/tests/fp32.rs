use dfdx::prelude::{Module as DfdxModule, *};
use itertools::Itertools;
use rand::{rngs::StdRng, Rng, SeedableRng};

use luminal::{
    nn::{activation::ReLU, linear::Linear},
    prelude::{Module, *},
    tests::{assert_close, assert_close_precision, random_vec, random_vec_rng},
};

use crate::MetalCompiler;

#[test]
fn test_contiguous() {
    let mut cx = Graph::new();
    let data = random_vec(12);
    let a = cx.tensor::<R2<3, 4>>().set(data.clone());
    let mut b = a.permute::<R2<4, 3>, _>().reshape::<R2<12, 1>>().retrieve();
    cx.compile(MetalCompiler::<f32>::default(), &mut b);
    cx.execute();

    let d_dev = Cpu::default();
    let d_a = d_dev.tensor_from_vec(data, (dfdx::shapes::Const::<3>, dfdx::shapes::Const::<4>));
    let d_b = d_a.permute::<Rank2<4, 3>, _>().reshape::<Rank2<12, 1>>();

    assert_close(&b.data(), &d_b.as_vec());
}

#[test]
fn test_log2() {
    let mut cx = Graph::new();
    let data = random_vec(3);
    let a = cx.tensor::<R1<3>>().set(data.clone());
    let mut b = a.log2().retrieve();

    cx.compile(MetalCompiler::<f32>::default(), &mut b);
    cx.execute();

    assert_close(
        &b.data(),
        &data.into_iter().map(|i: f32| i.log2()).collect::<Vec<_>>(),
    );
}

#[test]
fn test_exp2() {
    let mut cx = Graph::new();
    let data = random_vec(3);
    let a = cx.tensor::<R1<3>>().set(data.clone());
    let mut b = a.exp2().retrieve();

    cx.compile(MetalCompiler::<f32>::default(), &mut b);
    cx.execute();

    assert_close(
        &b.data(),
        &data.into_iter().map(|i: f32| i.exp2()).collect::<Vec<_>>(),
    );
}

#[test]
fn test_recip() {
    let mut cx = Graph::new();
    let a = cx.tensor::<R1<3>>().set(vec![1., 2., 4096.]);
    let mut b = a.recip().retrieve();
    cx.compile(MetalCompiler::<f32>::default(), &mut b);
    cx.execute();

    let d_dev = Cpu::default();
    let d_a = d_dev.tensor([1., 2., 4096.]);
    let d_b = d_a.recip();

    assert_close(&b.data(), &d_b.to_dtype::<f32>().as_vec());
}

#[test]
fn test_sin() {
    let mut cx = Graph::new();
    let a = cx.tensor::<R1<3>>().set(vec![1., 2., 3.]);
    let mut b = a.sin().retrieve();
    cx.compile(MetalCompiler::<f32>::default(), &mut b);
    cx.execute();

    let d_dev = Cpu::default();
    let d_a = d_dev.tensor([1., 2., 3.]);
    let d_b = d_a.sin();

    assert_close(&b.data(), &d_b.to_dtype::<f32>().as_vec());
}

#[test]
fn test_sqrt() {
    let mut cx = Graph::new();
    let a = cx.tensor::<R1<3>>().set(vec![1., 2., 3.]);
    let mut b = a.sqrt().retrieve();
    cx.compile(MetalCompiler::<f32>::default(), &mut b);
    cx.execute();

    let d_dev = Cpu::default();
    let d_a = d_dev.tensor([1., 2., 3.]);
    let d_b = d_a.sqrt();

    assert_close(&b.data(), &d_b.to_dtype::<f32>().as_vec());
}

#[test]
fn test_add() {
    let mut cx = Graph::new();
    let a = cx.tensor::<R1<3>>().set(vec![1., 2., 3.]);
    let b = cx.tensor::<R1<3>>().set(vec![1., 2., 3.]);
    let mut c = a + b;
    c.retrieve();

    cx.compile(MetalCompiler::<f32>::default(), &mut c);
    cx.execute();

    let d_dev = Cpu::default();
    let d_a = d_dev.tensor([1., 2., 3.]);
    let d_b = d_dev.tensor([1., 2., 3.]);
    let d_c = d_a + d_b;

    assert_close(&c.data(), &d_c.as_vec());
}

#[test]
fn test_sub() {
    let mut cx = Graph::new();
    let a = cx.tensor::<R1<3>>().set(vec![1., 2., 3.]);
    let b = cx.tensor::<R1<3>>().set(vec![1., 2., 3.]);
    let mut c = a - b;
    c.retrieve();

    cx.compile(MetalCompiler::<f32>::default(), &mut c);
    cx.execute();

    let d_dev = Cpu::default();
    let d_a = d_dev.tensor([1., 2., 3.]);
    let d_b = d_dev.tensor([1., 2., 3.]);
    let d_c = d_a - d_b;

    assert_close(&c.data(), &d_c.as_vec());
}

#[test]
fn test_square() {
    let mut cx = Graph::new();
    let mut rng = rand::thread_rng();
    let data = (0..40960)
        .map(|_| rng.gen_range(-0.01..0.01))
        .collect::<Vec<f32>>();
    let a = cx
        .tensor::<(Dyn<'b'>, Dyn<'s'>, luminal::prelude::Const<4096>)>()
        .set_dyn(data.clone(), &[1, 10, 4096]);
    let mut b = a * a;
    b.retrieve();

    cx.compile(<(GenericCompiler, MetalCompiler<f32>)>::default(), &mut b);
    cx.execute();

    let d_dev = Cpu::default();
    let d_a = d_dev.tensor_from_vec::<Rank3<1, 10, 4096>>(
        data,
        (
            dfdx::prelude::Const::<1>,
            dfdx::prelude::Const::<10>,
            dfdx::prelude::Const::<4096>,
        ),
    );
    let d_b = d_a.clone() * d_a;

    assert_close(&b.data(), &d_b.as_vec());
}

#[test]
fn test_mul() {
    let mut cx = Graph::new();
    let a = cx.tensor::<R1<3>>().set(vec![1., 2., 3.]);
    let b = cx.tensor::<R1<3>>().set(vec![1., 2., 3.]);
    let mut c = a * b;
    c.retrieve();

    cx.compile(MetalCompiler::<f32>::default(), &mut c);
    cx.execute();

    let d_dev = Cpu::default();
    let d_a = d_dev.tensor([1., 2., 3.]);
    let d_b = d_dev.tensor([1., 2., 3.]);
    let d_c = d_a * d_b;

    assert_close(&c.data(), &d_c.as_vec());
}

#[test]
fn test_mul2() {
    let mut cx = Graph::new();
    let a = cx
        .tensor::<(
            luminal::prelude::Const<1>,
            luminal::prelude::Const<1>,
            Dyn<'a'>,
            Dyn<'a'>,
        )>()
        .set_dyn(vec![82.4, 783.0, 99.6, 974.5], &[1, 1, 2, 2]);
    let b = cx.tensor::<R0>().set(vec![0.57735026]);
    let mut c = a * b.expand();
    c.retrieve();

    cx.compile(MetalCompiler::<f32>::default(), &mut c);
    cx.execute();

    let d_dev = Cpu::default();
    let d_a = d_dev.tensor([[[[82.4, 783.0], [99.6, 974.5]]]]);
    let d_b = d_dev.tensor(0.57735026);
    let d_c = d_a * d_b.broadcast::<_, dfdx::shapes::Axes4<0, 1, 2, 3>>();

    assert_close(&c.data(), &d_c.as_vec());
}

#[test]
fn test_div() {
    let mut cx = Graph::new();
    let a = cx.tensor::<R1<3>>().set(vec![1., 2., 3.]);
    let b = cx.tensor::<R1<3>>().set(vec![1., 2., 3.]);
    let mut c = a / b;
    c.retrieve();

    cx.compile(MetalCompiler::<f32>::default(), &mut c);
    cx.execute();

    let d_dev = Cpu::default();
    let d_a = d_dev.tensor([1., 2., 3.]);
    let d_b = d_dev.tensor([1., 2., 3.]);
    let d_c = d_a / d_b;

    assert_close(&c.data(), &d_c.as_vec());
}

#[test]
fn test_max() {
    let mut cx = Graph::new();
    let a = cx.tensor::<R1<3>>().set(vec![1., 2., 3.]);
    let b = cx.tensor::<R1<3>>().set(vec![1., 2., 3.]);
    let mut c = a.max(b).retrieve();

    cx.compile(MetalCompiler::<f32>::default(), &mut c);
    cx.execute();

    let d_dev = Cpu::default();
    let d_a = d_dev.tensor([1., 2., 3.]);
    let d_b = d_dev.tensor([1., 2., 3.]);
    let d_c = d_a.maximum(d_b);

    assert_close(&c.data(), &d_c.as_vec());
}

#[test]
fn test_mod() {
    let mut cx = Graph::new();
    let a_data = random_vec(3);
    let b_data = random_vec(3);
    let a = cx.tensor::<R1<3>>().set(a_data.clone());
    let b = cx.tensor::<R1<3>>().set(b_data.clone());
    let mut c = a % b;
    c.retrieve();

    cx.compile(MetalCompiler::<f32>::default(), &mut c);
    cx.execute();

    // No dfdx equivalent

    assert_close(
        &c.data(),
        &a_data
            .into_iter()
            .zip(b_data)
            .map(|(a, b)| a % b)
            .collect_vec(),
    );
}

// Reduction op tests

#[test]
fn test_sum_reduce() {
    let mut cx = Graph::new();
    let data = random_vec(4096);
    let a = cx.tensor::<R3<1, 1, 4096>>();
    a.set(data.clone());
    let mut b = a.sum_reduce::<_, luminal::prelude::Axis<1>>().retrieve();
    let mut c = a.sum_reduce::<_, luminal::prelude::Axis<0>>().retrieve();
    let mut d = a.sum_reduce::<_, luminal::prelude::Axis<2>>().retrieve();

    cx.compile(MetalCompiler::<f32>::default(), (&mut b, &mut c, &mut d));
    cx.execute();

    let d_dev = Cpu::default();
    let d_a = d_dev.tensor_from_vec(
        data,
        (
            dfdx::shapes::Const::<1>,
            dfdx::shapes::Const::<1>,
            dfdx::shapes::Const::<4096>,
        ),
    );
    let d_b = d_a.clone().sum::<_, dfdx::shapes::Axis<1>>();
    let d_c = d_a.clone().sum::<_, dfdx::shapes::Axis<0>>();
    let d_d = d_a.sum::<_, dfdx::shapes::Axis<2>>();

    assert_close(&b.data(), &d_b.as_vec());
    assert_close(&c.data(), &d_c.as_vec());
    assert_close(&d.data(), &d_d.as_vec());
}

#[test]
fn test_max_reduce() {
    let mut cx = Graph::new();
    let data = random_vec(12);
    let a = cx.tensor::<R3<2, 2, 3>>();
    a.set(data.clone());
    let mut b = a.max_reduce::<_, luminal::prelude::Axis<1>>().retrieve();
    let mut c = a.max_reduce::<_, luminal::prelude::Axis<0>>().retrieve();
    let mut d = a.max_reduce::<_, luminal::prelude::Axis<2>>().retrieve();

    cx.compile(MetalCompiler::<f32>::default(), (&mut b, &mut c, &mut d));
    cx.execute();

    let d_dev = Cpu::default();
    let d_a = d_dev.tensor_from_vec(
        data,
        (
            dfdx::shapes::Const::<2>,
            dfdx::shapes::Const::<2>,
            dfdx::shapes::Const::<3>,
        ),
    );
    let d_b = d_a.clone().max::<_, dfdx::shapes::Axis<1>>();
    let d_c = d_a.clone().max::<_, dfdx::shapes::Axis<0>>();
    let d_d = d_a.max::<_, dfdx::shapes::Axis<2>>();

    assert_close(&b.data(), &d_b.as_vec());
    assert_close(&c.data(), &d_c.as_vec());
    assert_close(&d.data(), &d_d.as_vec());
}

#[test]
fn test_mean_reduce() {
    let data = random_vec(40960);
    let mut cx = Graph::new();
    let a = cx.tensor::<R3<1, 10, 4096>>().set(data.clone());
    let mut b = a.mean_reduce::<_, luminal::prelude::Axis<2>>().retrieve();

    cx.compile(MetalCompiler::<f32>::default(), &mut b);
    cx.execute();

    let d_dev = Cpu::default();
    let d_a = d_dev.tensor_from_vec(
        data,
        (
            dfdx::shapes::Const::<1>,
            dfdx::shapes::Const::<10>,
            dfdx::shapes::Const::<4096>,
        ),
    );
    let d_b = d_a.mean::<_, dfdx::shapes::Axis<2>>();
    assert_close(&b.data(), &d_b.as_vec());
}

#[test]
fn test_matmul_simple() {
    let mut cx = Graph::new();
    let a_data = random_vec(256 * 256);
    let b_data = random_vec(256 * 256);
    let a = cx.tensor::<R2<256, 256>>().set(a_data.clone());
    let b = cx.tensor::<R2<256, 256>>().set(b_data.clone());
    let mut c = a.matmul(b).retrieve();

    cx.compile(MetalCompiler::<f32>::default(), &mut c);
    cx.execute();

    let d_dev = Cpu::default();
    let d_a = d_dev.tensor_from_vec(
        a_data,
        (dfdx::shapes::Const::<256>, dfdx::shapes::Const::<256>),
    );
    let d_b = d_dev.tensor_from_vec(
        b_data,
        (dfdx::shapes::Const::<256>, dfdx::shapes::Const::<256>),
    );
    let d_c = d_a.matmul(d_b);

    assert_close(&c.data(), &d_c.as_vec());
}

#[test]
fn test_matmul() {
    let mut cx = Graph::new();
    let a_data = random_vec(512 * 512);
    let b_data = random_vec(512 * 512);
    let a = cx.tensor::<R2<512, 512>>().set(a_data.clone());
    let b = cx.tensor::<R2<512, 512>>().set(b_data.clone());
    let mut c = a.matmul(b).retrieve();

    cx.compile(MetalCompiler::<f32>::default(), &mut c);
    cx.execute();

    let d_dev = Cpu::default();
    let d_a = d_dev.tensor_from_vec(
        a_data,
        (dfdx::shapes::Const::<512>, dfdx::shapes::Const::<512>),
    );
    let d_b = d_dev.tensor_from_vec(
        b_data,
        (dfdx::shapes::Const::<512>, dfdx::shapes::Const::<512>),
    );
    let d_c = d_a.matmul(d_b);

    assert_close(&c.data(), &d_c.as_vec());
}

#[test]
fn test_batch_matmul() {
    let mut cx = Graph::new();
    let a = cx
        .tensor::<R3<2, 2, 3>>()
        .set(vec![1., 2., 3., 1., 2., 1., 1., 2., 3., 1., 2., 1.]);
    let b = cx
        .tensor::<R2<3, 4>>()
        .set(vec![1., 2., 3., 1., 1., 2., 1., 2., -1., -2., 1., 2.]);
    let mut c = a.matmul(b).retrieve();

    cx.compile(MetalCompiler::<f32>::default(), &mut c);
    cx.execute();

    let d_dev = Cpu::default();
    let d_a = d_dev.tensor([[[1., 2., 3.], [1., 2., 1.]], [[1., 2., 3.], [1., 2., 1.]]]);
    let d_b = d_dev.tensor([[1., 2., 3., 1.], [1., 2., 1., 2.], [-1., -2., 1., 2.]]);
    let d_c = d_a.matmul(d_b);

    assert_close(&c.data(), &d_c.as_vec());
}

#[test]
fn test_matmul_transpose() {
    const M: usize = 1024; // Any
    const K: usize = 16; // >= 16
    const N: usize = 256; // >= 256, power of 2
    let mut cx = Graph::new();
    let mut rng = StdRng::seed_from_u64(0);

    let a_data = random_vec_rng(M * K, &mut rng);
    let a = cx.tensor::<R2<M, K>>().set(a_data.clone());
    let b_data = random_vec_rng(K * N, &mut rng);
    let b = cx.tensor::<R2<N, K>>().set(b_data.clone());
    let a_t_data = random_vec_rng(K * M, &mut rng);
    let a_t = cx.tensor::<R2<K, M>>().set(a_t_data.clone());
    let b_t_data = random_vec_rng(K * N, &mut rng);
    let b_t = cx.tensor::<R2<K, N>>().set(b_t_data.clone());

    let mut a_b = a.matmul(b.permute()).retrieve();
    let mut a_b_t = a.matmul(b_t).retrieve();
    let mut a_t_b = a_t
        .permute::<_, luminal::prelude::Axes2<1, 0>>()
        .matmul(b.permute())
        .retrieve();
    let mut a_t_b_t = a_t
        .permute::<_, luminal::prelude::Axes2<1, 0>>()
        .matmul(b_t)
        .retrieve();

    cx.compile(
        MetalCompiler::<f32>::default(),
        (&mut a_b, &mut a_b_t, &mut a_t_b, &mut a_t_b_t),
    );
    cx.execute();

    let d_dev = Cpu::default();
    let d_a = d_dev.tensor_from_vec(a_data, (dfdx::shapes::Const::<M>, dfdx::shapes::Const::<K>));
    let d_b = d_dev.tensor_from_vec(b_data, (dfdx::shapes::Const::<N>, dfdx::shapes::Const::<K>));
    let d_a_t = d_dev.tensor_from_vec(
        a_t_data,
        (dfdx::shapes::Const::<K>, dfdx::shapes::Const::<M>),
    );
    let d_b_t = d_dev.tensor_from_vec(
        b_t_data,
        (dfdx::shapes::Const::<K>, dfdx::shapes::Const::<N>),
    );
    let d_a_b = d_a.clone().matmul(d_b.clone().permute());
    let d_a_b_t = d_a.matmul(d_b_t.clone());
    let d_a_t_b = d_a_t
        .clone()
        .permute::<_, dfdx::shapes::Axes2<1, 0>>()
        .matmul(d_b.permute());
    let d_a_t_b_t = d_a_t
        .permute::<_, dfdx::shapes::Axes2<1, 0>>()
        .matmul(d_b_t);

    assert_close(&a_b.data(), &d_a_b.as_vec());
    assert_close(&a_b_t.data(), &d_a_b_t.as_vec());
    assert_close(&a_t_b.data(), &d_a_t_b.as_vec());
    assert_close(&a_t_b_t.data(), &d_a_t_b_t.as_vec());
}

#[test]
fn test_relu_and_linear() {
    // Test single and batch, unoptimized and optimized
    let mut cx = Graph::new();
    let input_data = random_vec(32);
    let w1 = random_vec(32 * 64);
    let w2 = random_vec(32 * 64);
    let batch = cx
        .named_tensor::<R2<2, 32>>("Batch")
        .set(random_vec(32 * 2));
    let a = cx.named_tensor::<R1<32>>("Single").set(input_data.clone());

    let model: (Linear<32, 64>, ReLU, Linear<64, 32>) = InitModule::initialize(&mut cx);
    model.0.weight.set(w1.clone());
    model.2.weight.set(w2.clone());
    let mut b = model.forward(a).retrieve();
    let mut batch_out = model.forward(batch).retrieve();
    cx.execute();

    let unoptimized_b = b.data();
    let unoptimized_batch_out = batch_out.data();
    b.drop();
    batch_out.drop();
    cx.compile(
        <(GenericCompiler, MetalCompiler<f32>)>::default(),
        (&mut b, &mut batch_out),
    );
    cx.execute();

    assert_close_precision(&unoptimized_b, &b.data(), 2);
    assert_close_precision(&unoptimized_batch_out, &batch_out.data(), 2);

    // Test against dfdx
    let dev = Cpu::default();
    let mut model = <(
        dfdx::nn::modules::builders::UnbiasedLinear<32, 64>,
        dfdx::nn::modules::builders::ReLU,
        dfdx::nn::modules::builders::UnbiasedLinear<64, 32>,
    )>::build_on_device(&dev);
    // Set weights
    model.0.weight = dev
        .tensor_from_vec(w1, (dfdx::shapes::Const::<32>, dfdx::shapes::Const::<64>))
        .permute();
    model.2.weight = dev
        .tensor_from_vec(w2, (dfdx::shapes::Const::<64>, dfdx::shapes::Const::<32>))
        .permute();
    let a = dev.tensor_from_vec(input_data, (dfdx::shapes::Const::<32>,));
    let out = model.forward(a);

    assert_close_precision(&unoptimized_b, &out.as_vec(), 2);
}

#[test]
fn test_transformer_encoder_block() {
    let mut cx = Graph::new();
    let model: luminal::nn::transformer::encoder::TransformerEncoderBlock<3, 4, 1> =
        InitModule::initialize(&mut cx);
    model
        .attention
        .w_k
        .weight
        .set(vec![1., 22., 3., 1., 2., 3., 1., 2., 3.]);
    model
        .attention
        .w_q
        .weight
        .set(vec![3., 2., 3., 1.3, 2., 3., 3., 2., 3.]);
    model
        .attention
        .w_v
        .weight
        .set(vec![-1., 12., 3., -1., 2., -3., 11., 2., 3.]);
    model
        .attention
        .w_o
        .weight
        .set(vec![1., 22., 3., 1., 2., 3., 1., 2., 3.]);
    model
        .ff
        .0
        .weight
        .set(vec![-1., 12., 3., -1., 2., -3., 11., 2., 3., 11., 2., 3.]);
    model
        .ff
        .2
        .weight
        .set(vec![-1., 12., 3., -1., 2., -3., 11., 2., 3., 3., -1., 2.]);

    let a = cx
        .tensor::<(Dyn<'b'>, Dyn<'a'>, luminal::prelude::Const<3>)>()
        .set_dyn(vec![-1., 2., 3., 3., 3., -1.], &[1, 2, 3]);
    let mut b = model.forward(a).retrieve();

    cx.compile(<(GenericCompiler, MetalCompiler<f32>)>::default(), &mut b);
    cx.execute();

    let d_dev = Cpu::default();
    let mut d_model: dfdx::nn::modules::TransformerEncoderBlock<3, 1, 4, f32, Cpu> =
        d_dev.build_module::<dfdx::nn::modules::builders::TransformerEncoderBlock<3, 1, 4>, f32>();
    d_model.self_attn.w_k.bias.copy_from(&[0.0, 0.0, 0.0]);
    d_model.self_attn.w_v.bias.copy_from(&[0.0, 0.0, 0.0]);
    d_model.self_attn.w_q.bias.copy_from(&[0.0, 0.0, 0.0]);
    d_model.self_attn.w_o.bias.copy_from(&[0., 0., 0.]);
    d_model.self_attn.w_o.weight = d_dev
        .tensor_from_vec(
            vec![1., 22., 3., 1., 2., 3., 1., 2., 3.],
            (dfdx::shapes::Const::<3>, dfdx::shapes::Const::<3>),
        )
        .permute();
    d_model.self_attn.w_k.weight = d_dev
        .tensor_from_vec(
            vec![1., 22., 3., 1., 2., 3., 1., 2., 3.],
            (dfdx::shapes::Const::<3>, dfdx::shapes::Const::<3>),
        )
        .permute();
    d_model.self_attn.w_q.weight = d_dev
        .tensor_from_vec(
            vec![3., 2., 3., 1.3, 2., 3., 3., 2., 3.],
            (dfdx::shapes::Const::<3>, dfdx::shapes::Const::<3>),
        )
        .permute();
    d_model.self_attn.w_v.weight = d_dev
        .tensor_from_vec(
            vec![-1., 12., 3., -1., 2., -3., 11., 2., 3.],
            (dfdx::shapes::Const::<3>, dfdx::shapes::Const::<3>),
        )
        .permute();
    d_model.ff.0 .0.weight = d_dev
        .tensor_from_vec(
            vec![-1., 12., 3., -1., 2., -3., 11., 2., 3., 11., 2., 3.],
            (dfdx::shapes::Const::<3>, dfdx::shapes::Const::<4>),
        )
        .permute();
    d_model.ff.0 .0.bias = d_dev.tensor_from_vec(vec![0., 0., 0., 0.], (dfdx::shapes::Const::<4>,));
    d_model.ff.0 .2.weight = d_dev
        .tensor_from_vec(
            vec![-1., 12., 3., -1., 2., -3., 11., 2., 3., 3., -1., 2.],
            (dfdx::shapes::Const::<4>, dfdx::shapes::Const::<3>),
        )
        .permute();
    d_model.ff.0 .2.bias = d_dev.tensor_from_vec(vec![0., 0., 0.], (dfdx::shapes::Const::<3>,));
    d_model.norm1.gamma = d_dev.tensor_from_vec(vec![1., 1., 1.], (dfdx::shapes::Const::<3>,));
    d_model.norm2.gamma = d_dev.tensor_from_vec(vec![1., 1., 1.], (dfdx::shapes::Const::<3>,));
    d_model.norm1.epsilon = 1e-5;
    d_model.norm2.beta = d_dev.tensor_from_vec(vec![0., 0., 0.], (dfdx::shapes::Const::<3>,));
    d_model.norm1.beta = d_dev.tensor_from_vec(vec![0., 0., 0.], (dfdx::shapes::Const::<3>,));
    d_model.norm2.epsilon = 1e-5;
    let d_a = d_dev.tensor_from_vec(
        vec![-1., 2., 3., 3., 3., -1.],
        (dfdx::shapes::Const::<2>, dfdx::shapes::Const::<3>),
    );
    let d_b = d_model.forward(d_a);

    assert_close(&b.data(), &d_b.as_vec());
}
