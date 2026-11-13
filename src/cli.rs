use crate::{
    decoder::{decode_llr, dv_llr, parallel_nodes},
    encoder::encode_ra,
    graph::Graph,
    input::{parse_bit_frames, parse_bits},
};
use std::{env, fs, time::Instant};

const HELP: &str = r#"ra-reconcile — декодер обратной согласованности для двоичных RA-LDPC кодов

Программа реализует sum-product belief propagation (BP), используемый, чтобы по
своей шумной строке A и переданному сообщению M восстановить кодовое слово C.
В протоколе M = C XOR B, а A и B отличаются с вероятностью p. Результат содержит
оценку C, признак выполнения проверок чётности и число итераций.

Использование:
  ra-reconcile decode --matrix H.edges --alice 0101 --message 0011 --p 0.1 [опции]
  ra-reconcile decode-cv --matrix H.edges --x x.txt --soft m.txt --snr-db 5 [опции]
  ra-reconcile decode-batch --matrix H.edges --alice-file A.frames --message-file M.frames --p 0.1
  ra-reconcile encode --matrix H.edges --message 1010 [--output codeword.txt]

Параметры:
  --matrix FILE    Разреженный граф H в текстовом формате: первая строка «m n»,
                   далее по одному ребру на строку: «индекс_проверки индекс_бита».
                   Индексы начинаются с нуля. H задаёт проверку H*C = 0 (mod 2).
  --alice BITS     Известная получателю шумная бинарная строка A длины n.
  --message BITS   Для decode: публичное M=C XOR B, длины n. Для encode: k битов ключа.
  --p FLOAT        Для decode/decode-batch: оценка ошибки A и B, 0 < p < 0.5.
  --max-iter N     Максимум итераций BP (по умолчанию 100).
  --threads N      Потоки узлов в decode; потоки кадров в decode-batch (по умолчанию все CPU).
  --alice-file F   Читать длинную строку A из файла вместо --alice.
  --message-file F Читать M из файла; для encode это k-битовое сообщение.
                   В decode-batch каждый непустой ряд файла задаёт отдельный кадр.
  encode           Систематически закодировать k битов RA-матрицей (n−m проверочных битов).
  --x FILE         Для decode-cv: квадратуры X Алисы, по одному числу на строку.
  --soft FILE      Для decode-cv: знаковые мягкие наблюдения Боба, по одному числу на строку.
  --snr-db FLOAT   Для decode-cv: SNR канала в дБ; LLR=2·10^(SNR/10)·X·soft.
  --output FILE    Записать оценку кодового слова в файл; сводка остаётся в stdout.
                   Для decode-batch файл содержит по одной оценке на строку.
  -h, --help       Эта справка.

Пример: ra-reconcile decode --matrix H.edges --alice 001011 --message 101010 --p 0.08
Пример кодирования: ra-reconcile encode --matrix H.edges --message 1010
Коды завершения: 0 — синдром сошёлся, 2 — ввод/ошибка, 3 — лимит итераций без сходимости.
"#;
fn arg(args: &[String], key: &str, default: Option<&str>) -> Result<String, String> {
    args.windows(2)
        .find(|w| w[0] == key)
        .map(|w| w[1].clone())
        .or_else(|| default.map(str::to_string))
        .ok_or(format!("missing required option {key}"))
}
pub fn run() -> Result<i32, String> {
    let a: Vec<String> = env::args().collect();
    if a.len() == 1 || a.iter().any(|x| x == "--help" || x == "-h") {
        print!("{HELP}");
        return Ok(0);
    }
    let mode = a.get(1).map(String::as_str).unwrap_or("");
    if mode != "decode" && mode != "decode-cv" && mode != "decode-batch" && mode != "encode" {
        return Err(
            "expected subcommand: encode, decode, decode-cv, or decode-batch\nRun with --help for usage".into(),
        );
    }
    let matrix = arg(&a, "--matrix", None)?;
    let raw = fs::read_to_string(&matrix).map_err(|e| format!("cannot read {matrix}: {e}"))?;
    let mut lines = raw
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.starts_with('#'));
    let d: Vec<_> = lines
        .next()
        .ok_or("empty matrix")?
        .split_whitespace()
        .collect();
    if d.len() != 2 {
        return Err("matrix header must be: <check_nodes> <bits>".into());
    }
    let m: usize = d[0].parse().map_err(|_| "bad matrix row count")?;
    let n: usize = d[1].parse().map_err(|_| "bad matrix column count")?;
    let mut es = Vec::new();
    for line in lines {
        let x: Vec<_> = line.split_whitespace().collect();
        if x.len() != 2 {
            return Err("each edge line must contain row and column".into());
        }
        es.push((
            x[0].parse().map_err(|_| "bad row index")?,
            x[1].parse().map_err(|_| "bad column index")?,
        ));
    }
    // Timer starts after raw edge-list parsing, matching Octave's matrix load before BP.
    let started = Instant::now();
    let g = Graph::from_edges(m, n, &es)?;
    if mode == "encode" {
        if n <= m {
            return Err("RA matrix must have more columns than check rows".into());
        }
        let k = n - m;
        let message_raw = if a.iter().any(|x| x == "--message-file") {
            let path = arg(&a, "--message-file", None)?;
            fs::read_to_string(&path).map_err(|e| format!("cannot read --message-file: {e}"))?
        } else {
            arg(&a, "--message", None)?
        };
        let message = parse_bits(&message_raw, k, "--message")?;
        let codeword: String = encode_ra(&g, &message)?.iter().map(u8::to_string).collect();
        if a.iter().any(|x| x == "--output") {
            let path = arg(&a, "--output", None)?;
            fs::write(&path, format!("{codeword}\n"))
                .map_err(|e| format!("cannot write codeword to {path}: {e}"))?;
        } else {
            println!("codeword={codeword}");
        }
        return Ok(0);
    }
    let max: usize = arg(&a, "--max-iter", Some("100"))?
        .parse()
        .map_err(|_| "--max-iter must be integer")?;
    let default = std::thread::available_parallelism()
        .map(|v| v.get())
        .unwrap_or(1)
        .to_string();
    let threads: usize = arg(&a, "--threads", Some(&default))?
        .parse()
        .map_err(|_| "--threads must be integer")?;
    if threads == 0 || max == 0 {
        return Err("--threads and --max-iter must be positive".into());
    }
    if mode == "decode-batch" {
        let read_frames = |option: &str| -> Result<Vec<Vec<u8>>, String> {
            let path = arg(&a, option, None)?;
            let raw =
                fs::read_to_string(&path).map_err(|e| format!("cannot read {option}: {e}"))?;
            parse_bit_frames(&raw, n, option)
        };
        let alice = read_frames("--alice-file")?;
        let message = read_frames("--message-file")?;
        if alice.len() != message.len() {
            return Err(format!(
                "A contains {} frames, M contains {}",
                alice.len(),
                message.len()
            ));
        }
        let p: f64 = arg(&a, "--p", None)?
            .parse()
            .map_err(|_| "--p must be a number")?;
        if !(0.0..0.5).contains(&p) {
            return Err("--p must be strictly between 0 and 0.5".into());
        }
        let results = parallel_nodes(alice.len(), threads, |i| {
            let llr = dv_llr(&alice[i], &message[i], p);
            decode_llr(&g, &llr, max, 1)
        });
        let write_estimates = a.iter().any(|x| x == "--output");
        let mut estimates = Vec::with_capacity(results.len());
        let mut all_success = true;
        for (i, result) in results.into_iter().enumerate() {
            let (bits, success, iterations) = result?;
            all_success &= success;
            let estimate: String = bits.iter().map(u8::to_string).collect();
            if !write_estimates {
                println!("estimate_{}={estimate}", i + 1);
            }
            estimates.push(estimate);
            println!(
                "frame={} success={} iterations={}",
                i + 1,
                success,
                iterations
            );
        }
        if a.iter().any(|x| x == "--output") {
            let path = arg(&a, "--output", None)?;
            let content = estimates.join("\n");
            fs::write(&path, format!("{content}\n"))
                .map_err(|e| format!("cannot write batch estimates to {path}: {e}"))?;
        }
        println!(
            "frames={} all_success={} elapsed_ms={:.3}",
            estimates.len(),
            all_success,
            started.elapsed().as_secs_f64() * 1000.0
        );
        return Ok(if all_success { 0 } else { 3 });
    }
    let llr = if mode == "decode" {
        let read_bits = |inline: &str, file: &str, label: &str| -> Result<Vec<u8>, String> {
            let s = if a.iter().any(|x| x == file) {
                fs::read_to_string(arg(&a, file, None)?)
                    .map_err(|e| format!("cannot read {file}: {e}"))?
            } else {
                arg(&a, inline, None)?
            };
            parse_bits(&s, n, label)
        };
        let alice = read_bits("--alice", "--alice-file", "--alice")?;
        let msg = read_bits("--message", "--message-file", "--message")?;
        let p: f64 = arg(&a, "--p", None)?
            .parse()
            .map_err(|_| "--p must be a number")?;
        if !(0.0..0.5).contains(&p) {
            return Err("--p must be strictly between 0 and 0.5".into());
        }
        dv_llr(&alice, &msg, p)
    } else {
        let read_values = |key: &str| -> Result<Vec<f64>, String> {
            let path = arg(&a, key, None)?;
            let s = fs::read_to_string(&path).map_err(|e| format!("cannot read {key}: {e}"))?;
            s.split_whitespace()
                .map(|x| {
                    x.parse::<f64>()
                        .map_err(|_| format!("invalid number in {key}: {x}"))
                })
                .collect()
        };
        let x = read_values("--x")?;
        let soft = read_values("--soft")?;
        if x.len() != n || soft.len() != n {
            return Err(format!("--x and --soft each require {n} values"));
        }
        if x.iter().any(|v| !v.is_finite() || *v == 0.0) {
            return Err(
                "--x values must be finite and non-zero (the source CV formula divides by X)"
                    .into(),
            );
        }
        if soft.iter().any(|v| !v.is_finite()) {
            return Err("--soft values must be finite".into());
        }
        let snr: f64 = arg(&a, "--snr-db", None)?
            .parse()
            .map_err(|_| "--snr-db must be a number")?;
        if !snr.is_finite() {
            return Err("--snr-db must be finite".into());
        }
        let ratio = 10f64.powf(snr / 10.0);
        (0..n)
            .map(|j| 2.0 * ratio * x[j] * soft[j])
            .collect::<Vec<_>>()
    };
    let (c, ok, it) = decode_llr(&g, &llr, max, threads)?;
    let estimate: String = c.iter().map(u8::to_string).collect();
    if a.iter().any(|x| x == "--output") {
        let path = arg(&a, "--output", None)?;
        fs::write(&path, format!("{estimate}\n"))
            .map_err(|e| format!("cannot write estimate to {path}: {e}"))?;
    } else {
        println!("estimate={estimate}");
    }
    println!(
        "success={ok}\niterations={it}\nelapsed_ms={:.3}",
        started.elapsed().as_secs_f64() * 1000.0
    );
    Ok(if ok { 0 } else { 3 })
}
