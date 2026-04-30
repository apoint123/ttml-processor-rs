#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::{
    env,
    hint::black_box,
    path::Path,
    time::Duration,
};

use criterion::{
    Criterion,
    Throughput,
    criterion_group,
    criterion_main,
};
use ttml_processor::parse_ttml;

const SAMPLE_TTML: &str = include_str!("../tests/fixtures/緋色月下、狂咲ノ絶.722013.ttml");

fn benchmark_parse_ttml(c: &mut Criterion) {
    let mut group = c.benchmark_group("TTML Parsing");

    group.measurement_time(Duration::from_secs(20));
    group.sample_size(200);

    group.bench_function("parse_normal_ttml", |b| {
        b.iter(|| {
            let parsed_data = parse_ttml(black_box(SAMPLE_TTML)).expect("样本解析失败");

            black_box(parsed_data);
        });
    });

    group.finish();
}

fn benchmark_parse_amll_ttml_db(c: &mut Criterion) {
    let Ok(db_dir) = env::var("AMLL_TTML_DB_DIR") else {
        eprintln!("未设置环境变量 AMLL_TTML_DB_DIR，跳过此基准测试");
        return;
    };

    let dir = Path::new(&db_dir);

    let files: Vec<String> = match std::fs::read_dir(dir) {
        Ok(rd) => rd
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                path.extension()?
                    .eq_ignore_ascii_case("ttml")
                    .then(|| std::fs::read_to_string(&path).ok())?
            })
            .collect(),
        Err(e) => {
            eprintln!("无法读取目录 {db_dir}: {e}，跳过此基准测试");
            return;
        }
    };

    if files.is_empty() {
        eprintln!("目录 {db_dir} 中未找到任何 .ttml 文件，跳过此基准测试");
        return;
    }

    let total_bytes: u64 = files.iter().map(|s| s.len() as u64).sum();
    let file_count = files.len();

    let total_mib = total_bytes / (1024 * 1024);
    eprintln!("共加载 {file_count} 个文件，总计 {total_mib} MiB");

    let mut group = c.benchmark_group("AMLL TTML DB Parsing");
    group.throughput(Throughput::Bytes(total_bytes));
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(15));

    group.bench_function("parse_ttml_db", |b| {
        b.iter(|| {
            for content in &files {
                black_box(parse_ttml(black_box(content.as_str())).ok());
            }
        });
    });

    group.finish();
}

criterion_group!(benches, benchmark_parse_ttml, benchmark_parse_amll_ttml_db);

criterion_main!(benches);
