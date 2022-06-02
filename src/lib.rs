extern crate core;

// mod as_any;
mod error;
mod gen_factory;
mod rbase;
mod rbytes;
mod rcompress;
mod rcont;
mod riofs;
mod root;
pub mod rtypes;
mod utils;

pub use riofs::file;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
