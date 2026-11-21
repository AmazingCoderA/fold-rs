use fold::grid::build_map;
use fold::io::{parse, to_pgm};
use std::time::Instant;

fn main() {
    let mut args = std::env::args().skip(1);
    let first = match args.next() {
        Some(a) => a,
        None => {
            eprintln!("Fold — структурная карта отражающего горизонта");
            eprintln!("использование: fold <input.txt> [output.pgm]");
            eprintln!("               fold --bench");
            std::process::exit(2);
        }
    };
    if first == "--bench" {
        bench();
        return;
    }
    let input = first;
    let output = args.next().unwrap_or_else(|| "map.pgm".into());

    let text = match std::fs::read_to_string(&input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("не удалось прочитать {input}: {e}");
            std::process::exit(1);
        }
    };
    let (profiles, settings) = match parse(&text) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    let t = Instant::now();
    let grid = build_map(&profiles, &settings);
    let elapsed = t.elapsed();

    if let Err(e) = to_pgm(&grid, &output) {
        eprintln!("не удалось записать {output}: {e}");
        std::process::exit(1);
    }

    let filled = grid.data.iter().filter(|v| v.is_finite()).count();
    println!(
        "профилей: {}, отсчётов: {}, карта {}x{}, заполнено: {} ({:.1}%)",
        profiles.len(),
        profiles.iter().map(|p| p.picks.len()).sum::<usize>(),
        grid.width,
        grid.height,
        filled,
        100.0 * filled as f64 / grid.data.len() as f64
    );
    println!("время расчёта: {:.3} мс", elapsed.as_secs_f64() * 1000.0);
    println!("файл: {output}");
}

/// Стресс-тест: 15 профилей (максимум в UI оригинала) по 200 пиков каждый.
fn bench() {
    use fold::model::{Pick, Profile, Settings};

    let mut profiles = Vec::new();
    for p in 0..15 {
        let y0 = 200.0 + p as f64 * 257.0;
        let picks = (0..200)
            .map(|i| Pick {
                trace: i as f64,
                time: 0.7 + 0.4 * (i as f64 / 199.0) + 0.05 * (p as f64 * 0.7).sin(),
            })
            .collect();
        profiles.push(Profile {
            id: p,
            x0: 300.0,
            y0,
            x1: 7700.0,
            y1: y0 + 100.0,
            picks,
        });
    }
    let settings = Settings::default();
    let iterations = 50;
    let t = Instant::now();
    let mut sink = 0usize;
    for _ in 0..iterations {
        let g = build_map(&profiles, &settings);
        sink += g.data.iter().filter(|v| v.is_finite()).count();
    }
    let elapsed = t.elapsed();
    println!(
        "bench: {} профилей, {} пиков, карта {}x{}, {} итераций",
        profiles.len(),
        profiles.iter().map(|p| p.picks.len()).sum::<usize>(),
        settings.width,
        settings.height,
        iterations
    );
    println!(
        "среднее: {:.3} мс/построение (контрольная сумма {sink})",
        elapsed.as_secs_f64() * 1000.0 / iterations as f64
    );
}
