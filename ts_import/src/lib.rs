use std::{borrow::Cow, path::PathBuf};

use proc_macro2::Span;
use quote::{quote, quote_spanned};
use ts_parser::TsFile;
use unsynn::{
    unsynn, BraceGroupContaining, Comma, DelimitedVec, Error, Ident, LiteralString, Nothing, Parse,
    ToTokens, TokenIter, Transaction,
};

mod ts_parser;

unsynn! {
    keyword Pub = "pub";

    enum Visiblity {
        Public(Pub),
        Private(Nothing)
    }
}
impl quote::ToTokens for Visiblity {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        unsynn::ToTokens::to_tokens(self, tokens)
    }
}

unsynn! {
    keyword From = "from";

    struct TsImport {
        items: BraceGroupContaining<DelimitedVec<TsImportItem, Comma>>,
        from: From,
        path: LiteralString,
    }


    struct TsImportItem {
        vis: Visiblity,
        item: Ident,
    }
}

#[proc_macro]
pub fn import(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let call_site = Span::call_site();
    match import_impl(proc_macro2::TokenStream::from(item), call_site) {
        Ok(ts) => ts.into(),
        Err(e) => e.into_ts().into(),
    }
}

struct TsiError {
    message: Cow<'static, str>,
    span: Span,
}
impl TsiError {
    pub fn into_ts(self) -> proc_macro2::TokenStream {
        let span = self.span;
        let message = self.message.as_ref();
        quote_spanned! {span => compile_error!(#message); }
    }
}
impl std::convert::From<(&'static str, Span)> for TsiError {
    fn from(value: (&'static str, Span)) -> Self {
        Self {
            message: Cow::Borrowed(value.0),
            span: value.1,
        }
    }
}
impl std::convert::From<(String, Span)> for TsiError {
    fn from(value: (String, Span)) -> Self {
        Self {
            message: Cow::Owned(value.0),
            span: value.1,
        }
    }
}

fn read_ts_file(rel_path: &str, call_site: Span) -> Result<String, std::io::Error> {
    let mut full_path: PathBuf = call_site.local_file().unwrap_or(PathBuf::new());
    full_path.pop(); // remove the filename
    full_path.push(&rel_path);
    let ts_file_path = std::fs::canonicalize(full_path)?;
    Ok(std::fs::read_to_string(ts_file_path)?)
}

fn import_impl(
    item: proc_macro2::TokenStream,
    call_site: Span,
) -> Result<proc_macro2::TokenStream, TsiError> {
    let import = TsImport::parse(&mut item.into_token_iter())
        .map_err(|e| (format!("{}", e), call_site.clone()))?;
    let import_path = import.path.as_str().to_string();
    let import_path_span = import.path.into_inner().span();

    let ts_file_content = read_ts_file(&import_path, call_site)
        .map_err(|e| (format!("Failed to read file: {}", e), import_path_span))?;

    let ts_file = TsFile::parse(&mut ts_file_content.into_token_iter()).map_err(|e| {
        (
            format!("Failed to parse typescript: {}", e),
            call_site.clone(),
        )
    })?;

    let mut emitted_items = Vec::new();
    for item in import.items.content.0.iter() {
        let import_item = &item.value;
        let tsitem = ts_file
            .exported_items()
            .find(|tsitem| tsitem.item.name() == &import_item.item)
            .ok_or(("Couldn't find this item", import_item.item.span()))?;

        emitted_items.push(tsitem.item.emit(&import_item.vis));
    }

    Ok(quote! { const _: &str = include_str!(#import_path); #(#emitted_items)*})
}
