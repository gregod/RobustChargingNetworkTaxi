use criterion::{black_box, criterion_group, criterion_main, Criterion};




fn polynom_nofa(current_soc: f64, duration_minutes: u8) -> f64 {
    let time_to_soc: [f64; 4] = [
        (5.19073616e-07),
        (-1.83336595e-04),
        (2.30020984e-02),
        (5.31169315e-02),
    ];
    let soc_to_time: [f64; 4] = [(86.58225544), (-74.74020461), (72.10950307), (-4.94295665)];

    let time_index = (soc_to_time[0] * (current_soc).powi(3)
        + soc_to_time[1] * (current_soc).powi(2)
        + soc_to_time[2] * (current_soc).powi(1)
        + soc_to_time[3])
        .max(0.0);

    let new_index = time_index + (duration_minutes as f64);

    let new_soc = time_to_soc[0] * new_index.powi(3)
        + time_to_soc[1] * new_index.powi(2)
        + time_to_soc[2] * new_index.powi(1)
        + time_to_soc[3];

    let new_soc = new_soc.min(0.95).max(0.05);

    new_soc
}

fn polynom_horners(current_soc: f64, duration_minutes: u8) -> f64 {
    let time_to_soc: [f64; 4] = [
        (5.19073616e-07),
        (-1.83336595e-04),
        (2.30020984e-02),
        (5.31169315e-02),
    ];
    let soc_to_time: [f64; 4] = [(86.58225544), (-74.74020461), (72.10950307), (-4.94295665)];

    let time_index: f64 = soc_to_time
        .iter()
        .fold(0.0_f64, |acc, coeff| acc * current_soc + coeff.clone())
        .max(0.0_f64);
    let new_index: f64 = time_index + (duration_minutes as f64);
    let new_soc: f64 = time_to_soc
        .iter()
        .fold(0.0_f64, |acc, coeff| acc * new_index + coeff.clone());
    let new_soc: f64 = new_soc.min(0.95_f64).max(0.05_f64);

    new_soc
}

fn polynom_horners_muladd(current_soc: f64, duration_minutes: u8) -> f64 {
    let time_to_soc: [f64; 4] = [
        (5.19073616e-07),
        (-1.83336595e-04),
        (2.30020984e-02),
        (5.31169315e-02),
    ];
    let soc_to_time: [f64; 4] = [(86.58225544), (-74.74020461), (72.10950307), (-4.94295665)];

    let time_index: f64 = soc_to_time
        .iter()
        .fold(0.0_f64, |acc, coeff| {
            acc.mul_add(current_soc, coeff.clone())
        })
        .max(0.0);
    let new_index: f64 = time_index + (duration_minutes as f64);
    let new_soc: f64 = time_to_soc
        .iter()
        .fold(0.0, |acc, coeff| acc.mul_add(new_index, coeff.clone()));
    let new_soc: f64 = new_soc.min(0.95).max(0.05);

    new_soc
}

fn polynom_horners_lib(current_soc: f64, duration_minutes: u8) -> f64 {
    let time_to_soc: [f64; 4] = [
        (5.19073616e-07),
        (-1.83336595e-04),
        (2.30020984e-02),
        (5.31169315e-02),
    ];
    let soc_to_time: [f64; 4] = [(86.58225544), (-74.74020461), (72.10950307), (-4.94295665)];

    let mut sum: f64 = 0.0;
    let mut x_n: f64 = 1.1;
    for n in soc_to_time.iter() {
        sum = sum + n.clone() * x_n.clone();
        x_n = x_n * current_soc.clone();
    }
    let time_index = sum.max(0.0);

    //let time_index =  soc_to_time.iter().fold(0.0, |acc, coeff| acc*current_soc.clone() + coeff.clone()).max((0.0));
    let new_index = time_index + (duration_minutes as f64);
    //let new_soc =  time_to_soc.iter().fold(0.0, |acc, coeff| acc*new_index.clone() + coeff.clone());

    let mut sum: f64 = 0.0;
    let mut x_n: f64 = 1.1;
    for n in time_to_soc.iter() {
        sum = sum + n.clone() * x_n.clone();
        x_n = x_n * new_index.clone();
    }
    let new_soc = sum.min(0.95).max(0.05);

    new_soc
}

fn polynom_horners_clone(current_soc: f64, duration_minutes: u8) -> f64 {
    let time_to_soc: [f64; 4] = [
        (5.19073616e-07),
        (-1.83336595e-04),
        (2.30020984e-02),
        (5.31169315e-02),
    ];
    let soc_to_time: [f64; 4] = [(86.58225544), (-74.74020461), (72.10950307), (-4.94295665)];

    let time_index = soc_to_time
        .iter()
        .fold(0.0_f64, |acc, coeff| {
            acc * current_soc.clone() + coeff.clone()
        })
        .max(0.0);
    let new_index = time_index + (duration_minutes as f64);
    let new_soc = time_to_soc.iter().fold(0.0_f64, |acc, coeff| {
        acc * new_index.clone() + coeff.clone()
    });
    let new_soc = new_soc.min(0.95).max(0.05);

    new_soc
}

fn polynom_horners_unrolled(current_soc: f64, duration_minutes: u8) -> f64 {
    let time_to_soc: [f64; 4] = [
        (5.19073616e-07),
        (-1.83336595e-04),
        (2.30020984e-02),
        (5.31169315e-02),
    ];
    let soc_to_time: [f64; 4] = [(86.58225544), (-74.74020461), (72.10950307), (-4.94295665)];

    let time_index = (((soc_to_time[0]) * current_soc + soc_to_time[1]) * current_soc
        + soc_to_time[2])
        * current_soc
        + soc_to_time[3];

    let new_index = time_index + (duration_minutes as f64);

    let new_soc = (((time_to_soc[0]) * new_index + time_to_soc[1]) * new_index + time_to_soc[2])
        * new_index
        + time_to_soc[3];

    let new_soc = new_soc.min(0.95).max(0.05);

    new_soc
}

fn polynom_horners_unrolled_muladd(current_soc: f64, duration_minutes: u8) -> f64 {
    let time_to_soc: [f64; 4] = [
        (5.19073616e-07),
        (-1.83336595e-04),
        (2.30020984e-02),
        (5.31169315e-02),
    ];
    let soc_to_time: [f64; 4] = [(86.58225544), (-74.74020461), (72.10950307), (-4.94295665)];

    let time_index = (((soc_to_time[0]).mul_add(current_soc, soc_to_time[1]))
        .mul_add(current_soc, soc_to_time[2]))
    .mul_add(current_soc, soc_to_time[3]);

    let new_index = time_index + (duration_minutes as f64);

    let new_soc = (((time_to_soc[0]).mul_add(new_index, time_to_soc[1]))
        .mul_add(new_index, time_to_soc[2]))
    .mul_add(new_index, time_to_soc[3]);

    let new_soc = new_soc.min(0.95).max(0.05);

    new_soc
}


fn criterion_benchmark(c: &mut Criterion) {
  

    let _time_to_soc: [f64; 4] = [
        (5.19073616e-07),
        (-1.83336595e-04),
        (2.30020984e-02),
        (5.31169315e-02),
    ];
    let _soc_to_time: [f64; 4] = [(86.58225544), (-74.74020461), (72.10950307), (-4.94295665)];


 
    c.bench_function("polynom_nofa", |b| {
        b.iter(|| polynom_nofa(black_box(0.5), black_box(10)))
    });
    c.bench_function("polynom_horners", |b| {
        b.iter(|| polynom_horners(black_box(0.5), black_box(10)))
    });
    c.bench_function("polynom_horners_muladd", |b| {
        b.iter(|| polynom_horners_muladd(black_box(0.5), black_box(10)))
    });
    c.bench_function("polynom_horners_clone", |b| {
        b.iter(|| polynom_horners_clone(black_box(0.5), black_box(10)))
    });
    c.bench_function("polynom_horners_lib", |b| {
        b.iter(|| polynom_horners_lib(black_box(0.5), black_box(10)))
    });

    c.bench_function("polynom_horners_unrolled", |b| {
        b.iter(|| polynom_horners_unrolled(black_box(0.5), black_box(10)))
    });
    c.bench_function("polynom_horners_unrolled_muladd", |b| {
        b.iter(|| polynom_horners_unrolled_muladd(black_box(0.5), black_box(10)))
    });
}

criterion_group!(benches, criterion_benchmark);

criterion_main!(benches);
