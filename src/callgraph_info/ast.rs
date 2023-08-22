use super::ci::Rule;
use pest::Span;
use pest_ast::FromPest;

fn span_into_str(span: Span) -> &str {
    span.as_str()
}

#[derive(Debug, Clone, FromPest)]
#[pest_ast(rule(Rule::object))]
pub struct Object<'pest> {
    #[pest_ast(inner(with(span_into_str)))]
    pub kind: &'pest str,
    pub items: Vec<Item<'pest>>,
}

#[derive(Debug, Clone, FromPest)]
#[pest_ast(rule(Rule::item))]
pub enum Item<'pest> {
    Object(Object<'pest>),
    Field(Field<'pest>),
}

#[derive(Debug, Clone, FromPest)]
#[pest_ast(rule(Rule::field))]
pub struct Field<'pest> {
    #[pest_ast(inner(with(span_into_str)))]
    pub key: &'pest str,
    #[pest_ast(inner(with(span_into_str)))]
    pub value: &'pest str,
}
