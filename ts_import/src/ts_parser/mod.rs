use proc_macro2::TokenStream;
use unsynn::{unsynn, Any, Error, Ident, TokenIter, Transaction};

use crate::Visiblity;

mod ts_enum;
use ts_enum::TsEnum;

unsynn! {
    pub keyword Enum = "enum";
    pub keyword Export = "export";

    pub enum TsItem {
        Enum(TsEnum)
    }

    pub struct ExportedItem {
        export: Export,
        pub item: TsItem,
    }

    pub struct TsFile {
        pub exported_items: Any<ExportedItem>,
    }
}

impl TsItem {
    pub fn name(&self) -> &Ident {
        match self {
            Self::Enum(enumm) => &enumm.name,
        }
    }

    pub fn emit(&self, vis: &Visiblity) -> TokenStream {
        match self {
            Self::Enum(e) => e.emit(vis),
        }
    }
}
