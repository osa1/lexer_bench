use lexer_bench::lua::lexer_lexgen::Lexer as LuaLexgen;
use lexer_bench::lua::lexer_luster::Lexer as LuaLuster;
use lexer_bench::lua::lua_file_iter;

use std::fs;

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn generate_lua_code() -> String {
    let mut code = String::new();

    for file in lua_file_iter() {
        let file_contents = fs::read_to_string(file).expect("Unable to read test file");
        code.push_str(&file_contents);
    }

    for _ in 0..3 {
        let code_ = code.clone();
        code.push_str(&code_);
    }

    code
}

fn lua_benchmarks(c: &mut Criterion) {
    let code = generate_lua_code();

    c.bench_function("Lex Lua code -- luster", |b| {
        b.iter(|| {
            let mut lexer = LuaLuster::new(black_box(code.as_bytes()), |s| s.to_owned());
            loop {
                match lexer.read_token() {
                    Ok(Some(_token)) => {}
                    Ok(None) => break,
                    Err(err) => panic!("Error in luster benchmark: {}", err),
                }
            }
        })
    });

    c.bench_function("Lex Lua code -- lexgen", |b| {
        b.iter(|| {
            let mut lexer = LuaLexgen::new(black_box(&code));
            loop {
                match lexer.next() {
                    Some(Ok(_token)) => {}
                    Some(Err(err)) => panic!("Error in lexgen benchmark: {:?}", err),
                    None => break,
                }
            }
        })
    });
}

criterion_group!(benches, lua_benchmarks);
criterion_main!(benches);
