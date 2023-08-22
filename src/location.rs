use std::path::PathBuf;

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Location {
    file: PathBuf,
    line_num: usize,
    char_num: usize,
}

impl Location {
    pub fn parse(string: &str) -> Option<(Location, &str)> {
        let (maybe_path, maybe_rest) = match string.split_once(':') {
            Some(x) => x,
            None => return None,
        };
        let (path, rest) = match maybe_path.len() {
            1 => {
                // Windows has single-character drive letters followed by ':', so we have to handle that here
                let (path_rest, rest) = match maybe_rest.split_once(':') {
                    Some(x) => x,
                    None => return None,
                };
                (maybe_path.to_string() + ":" + path_rest, rest)
            }
            0 => return None,
            _ => (maybe_path.to_string(), maybe_rest),
        };

        let (line_num_str, rest) = rest
            .split_once(':')
            .unwrap_or_else(|| panic!("Error parsing line num of {rest}"));
        let (char_num_str, rest) = rest.split_once(':').unwrap_or((rest, ""));

        let line_num = line_num_str.parse::<usize>().unwrap();
        let char_num = char_num_str.parse::<usize>().unwrap();

        Some((
            Location {
                file: path.into(),
                line_num,
                char_num,
            },
            rest,
        ))
    }
}
