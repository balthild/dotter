use clap;
use parse;

use toml::value::Table;

use std::fs;
use std::io::{Read, Write};
use std::path::{Path};
use std::process;

use filesystem::{parse_path, ignore_absolute_join};

pub fn deploy(global: &clap::ArgMatches<'static>,
          specific: &clap::ArgMatches<'static>,
          verbosity: u64, act: bool) {

    // Configuration
    verb!(verbosity, 1, "Loading configuration...");
    let (files, variables) = load_configuration(global, verbosity);

    // Cache
    let cache = global.occurrences_of("nocache") == 0;
    verb!(verbosity, 1, "Cache: {}", cache);
    let cache_directory = specific.value_of("cache_directory").unwrap();
    if cache {
        verb!(verbosity, 1, "Creating cache directory at {}", cache_directory);
        if act && fs::create_dir_all(cache_directory).is_err() {
            println!("Failed to create cache directory.");
            process::exit(1);
        }
    }

    // Deploy files
    for pair in files {
        let from = &parse_path(&pair.0).unwrap();
        let to = &parse_path(pair.1.as_str().unwrap()).unwrap();
        verb!(verbosity, 1, "Deploying {:?} -> {:?}", from, to);
        deploy_file(from, to, &variables, verbosity,
                    act, cache, &parse_path(cache_directory).unwrap())
    }
}

fn deploy_file(from: &Path, to: &Path, variables: &Table,
               verbosity: u64, act: bool, cache: bool,
               cache_directory: &Path) {
    // Create target directory
    if act {
        let to_parent = to.parent().unwrap();
        if fs::create_dir_all(to_parent).is_err() {
            println!("Warning: Failed creating directory {:?}", to_parent);
        }
    }

    if cache {
        let to_cache = ignore_absolute_join(cache_directory, to);
        deploy_file(from, &to_cache, variables, verbosity,
                    act, false, cache_directory);
        verb!(verbosity, 1, "Copying {:?} to {:?}", to_cache, to);
        if act && fs::copy(&to_cache, to).is_err() {
            println!("Warning: Failed copying {:?} to {:?}",
                     to_cache, to);
        }
    } else {
        verb!(verbosity, 1, "Copying {:?} to {:?}", from, to);
        // TODO fix this
        // let mode = fs::metadata(src).unwrap();
        let mut content = String::new();
        if act {
            if let Ok(mut f_from) = fs::File::open(from) {
                if f_from.read_to_string(&mut content).is_err() {
                    // TODO: this warns about dirs, can maybe use?
                    // there's also fs::FileType <28-05-17, Amit Gold> //
                    println!("Warning: Couldn't read from {:?}", from);
                    return;
                }
            content = substitute_variables(content, variables);
            } else {
                println!("Warning: Failed to open {:?} for reading", from);
                return;
            }
        }
        if act {
            if let Ok(mut f_to) = fs::File::create(to) {
                if f_to.write_all(content.as_bytes()).is_err() {
                    println!("Warning: Couldn't write to {:?}", to);
                    return;
                }
                // [TODO]: f_to.set_mode(mode);
            } else {
                println!("Warning: Failed to open {:?} for writing", to);
                return;
            }
        }
    }
}

fn load_configuration(matches: &clap::ArgMatches<'static>,
              verbosity: u64) -> (Table, Table) {
    verb!(verbosity, 3, "Deploy args: {:?}", matches);

    // Load files
    let files: Table = parse::load_file(
            matches.value_of("files")
            .unwrap()).unwrap();
    verb!(verbosity, 2, "Files: {:?}", files);

    // Load variables
    let mut variables: Table = parse::load_file(
            matches.value_of("variables")
            .unwrap()).unwrap();
    verb!(verbosity, 2, "Variables: {:?}", variables);

    // Load secrets
    let mut secrets: Table = parse::load_file(
            matches.value_of("secrets")
            .unwrap()).unwrap();
    verb!(verbosity, 2, "Secrets: {:?}", secrets);

    variables.append(&mut secrets); // Secrets is now empty

    verb!(verbosity, 2, "Variables with secrets: {:?}", variables);

    (files, variables)
}

fn substitute_variables(content: String, variables: &Table) -> String {
    let mut content = content;
    for variable in variables {
        content = content.replace(&["{{ ", variable.0, " }}"].concat(),
                                  variable.1.as_str().unwrap());
    }
    content.to_string()
}
