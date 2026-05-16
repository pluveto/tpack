use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Spacing, TokenStream, TokenTree};

pub(crate) fn compile_error(message: &str) -> TokenStream {
    let mut stream = TokenStream::new();
    stream.extend([TokenTree::Ident(Ident::new(
        "compile_error",
        proc_macro::Span::call_site(),
    ))]);
    stream.extend([TokenTree::Punct(Punct::new('!', Spacing::Alone))]);
    let mut inner = TokenStream::new();
    inner.extend([TokenTree::Literal(Literal::string(message))]);
    stream.extend([TokenTree::Group(Group::new(Delimiter::Parenthesis, inner))]);
    stream
}
