use criterion::{Criterion, criterion_group, criterion_main};
use cw_games::stellaris::BaseGame;
use cw_model::{CaseInsensitiveInterner, LoadMode};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn benchmark_load_modes(c: &mut Criterion) {
    let mut group = c.benchmark_group("load_base_game");
    group.sampling_mode(criterion::SamplingMode::Flat);
    group.sample_size(10);

    group.bench_function("serial", |b| {
        b.iter(|| {
            let install_path = BaseGame::get_install_directory_windows().unwrap();
            let interner = CaseInsensitiveInterner::new();
            BaseGame::load_as_mod_definition(Some(&install_path), LoadMode::Serial, &interner)
        })
    });

    group.bench_function("parallel", |b| {
        b.iter(|| {
            let install_path = BaseGame::get_install_directory_windows().unwrap();
            let interner = CaseInsensitiveInterner::new();
            BaseGame::load_as_mod_definition(Some(&install_path), LoadMode::Parallel, &interner)
        })
    });

    group.finish();
}

criterion_group!(benches, benchmark_load_modes);
criterion_main!(benches);
