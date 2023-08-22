use crate::location::Location;

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Qualifier {
    Static,
    Dynamic,
    Bounded,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct StackUsage {
    pub function: Function,
    pub stack_usage: usize,
    pub qualifiers: Vec<Qualifier>,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Function {
    pub location: Location,
    pub name: String,
}

impl StackUsage {
    pub fn parse(string: &str) -> StackUsage {
        let split: Vec<&str> = string.split("\t").collect();
        let [function_name_str, usage_str, qualifier_str] = split[..]
        else {
            panic!("Malformed stack usage line: {string}")
        };

        let function = Function::parse(function_name_str);
        let stack_usage = usage_str.parse::<usize>().unwrap();
        let qualifiers: Vec<Qualifier> = qualifier_str
            .split(",")
            .map(|q| match q {
                "static" => Qualifier::Static,
                "dynamic" => Qualifier::Dynamic,
                "bounded" => Qualifier::Bounded,
                &_ => unreachable!()
            })
            .collect();

        StackUsage {
            function,
            stack_usage,
            qualifiers,
        }
    }
}


impl Function {
    pub fn parse(string: &str) -> Function {
        let (location, name) = Location::parse(string).unwrap();
        let name = name.to_string();

        Function { location, name }
    }
}