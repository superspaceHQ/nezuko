use std::path::PathBuf;

use tree_sitter_stack_graphs::{
    cli::{
        index::IndexArgs,
        query::{Definition, QueryArgs, Target},
        util::SourcePosition,
    },
    loader::{LanguageConfiguration, Loader},
    NoCancellation,
};
use tree_sitter_stack_graphs_python::language_configuration;

fn get_language_configurations(language: &str) -> Vec<LanguageConfiguration> {
    match language {
        "Python" => vec![language_configuration(&NoCancellation)],
        _ => vec![],
    }
}

fn get_sqlite_path() -> PathBuf {
    let current_dir = match std::env::current_dir() {
        Ok(path) => path,
        Err(e) => {
            println!("Error getting the current directory: {}", e);
            std::process::exit(1);
        }
    };
    let directory = current_dir.parent().unwrap_or(&current_dir);
    directory.join(format!("{}.sqlite", env!("CARGO_PKG_NAME")))
}

pub fn index_files(files: Vec<PathBuf>, language: &str) -> Result<(), anyhow::Error> {
    let language_configurations = get_language_configurations(language);

    let index_args = IndexArgs {
        source_paths: files,
        continue_from: None,
        verbose: true,
        hide_error_details: false,
        max_file_time: None,
        wait_at_start: false,
        stats: true,
        force: true,
    };

    let default_db_path = get_sqlite_path();
    let loader = Loader::from_language_configurations(language_configurations, None)
        .expect("Expected loader");

    log::info!(
        "Starting graph infexing inside {} \n",
        default_db_path.display()
    );

    index_args.run(&default_db_path, loader)
}

pub fn find_definition(file: PathBuf, line: u32, column: u32) -> Result<(), anyhow::Error> {
    let source_positions = vec![SourcePosition {
        path: file,
        line: line.try_into().unwrap(),
        column: column.try_into().unwrap(),
    }];

    let query_args = QueryArgs {
        wait_at_start: false,
        stats: true,
        target: Target::Definition(Definition {
            references: source_positions,
        }),
    };

    let db_path = get_sqlite_path();

    log::info!("Looking for definitions inside {} \n", db_path.display());

    query_args.run(&db_path)
}
