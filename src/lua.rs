// Caveats:
//
// - Some of the files in Lua test suite are not UTF-8. Those files are converted into UTF-8 as
//   that's easier to deal with in lexgen.
//
// - Shebang lines (`#!../lua`) removed from Lua files.

pub mod error;
pub mod lexer_lexgen;
pub mod lexer_luster;
pub mod token;

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs::{read_dir, read_to_string};
    use std::path::PathBuf;

    static LUA_TEST_FILES_DIR: &str = "test_files/lua";

    fn lua_file_iter() -> impl Iterator<Item = PathBuf> {
        let dir = read_dir(LUA_TEST_FILES_DIR).expect("Unable to read test_files/lua");
        dir.filter_map(|entry| {
            let entry = entry.expect("Unable to read dir entry");
            let path = entry.path();
            let extension = match path.extension() {
                None => return None,
                Some(ext) => ext,
            };

            if extension.eq_ignore_ascii_case("lua") {
                Some(path)
            } else {
                None
            }
        })
    }

    #[test]
    fn luster() {
        use lexer_luster::Lexer;

        let mut n_files = 0;
        let mut n_tokens = 0;

        for lua_file in lua_file_iter() {
            n_files += 1;

            println!("{}", lua_file.to_string_lossy());

            let file_contents = read_to_string(lua_file).expect("Unable to read test file");

            let mut lexer = Lexer::new(file_contents.as_bytes(), |slice| {
                slice.to_vec().into_boxed_slice()
            });

            loop {
                match lexer.read_token() {
                    Ok(Some(_token)) => n_tokens += 1,
                    Ok(None) => break,
                    Err(err) => panic!("Lexer error: {}", err),
                }
            }
        }

        if n_files == 0 {
            panic!(
                "{} is empty, did you forget `git submodule update --init`?",
                LUA_TEST_FILES_DIR
            );
        }

        println!("Generated {} tokens from {} files", n_tokens, n_files);
    }
}
