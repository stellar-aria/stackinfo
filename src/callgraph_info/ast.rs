use super::ci::Rule;
use pest::Span;
use pest_ast::FromPest;

fn span_into_str(span: Span) -> &str {
    span.as_str()
}

#[derive(Debug, Clone, FromPest)]
#[pest_ast(rule(Rule::object))]
pub struct Object {
    #[pest_ast(inner(with(span_into_str), with(str::parse), with(Result::unwrap)))]
    pub kind: String,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, FromPest)]
#[pest_ast(rule(Rule::item))]
pub enum Item {
    Object(Object),
    Field(Field),
}

#[derive(Debug, Clone, FromPest)]
#[pest_ast(rule(Rule::field))]
pub struct Field {
    #[pest_ast(inner(with(span_into_str), with(str::parse), with(Result::unwrap)))]
    pub key: String,
    #[pest_ast(inner(with(span_into_str), with(str::parse), with(Result::unwrap)))]
    pub value: String,
}