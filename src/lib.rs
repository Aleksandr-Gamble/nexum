pub mod core;
pub mod hashit;
pub mod opensearch;
pub mod postgres;
pub mod redis;
pub mod sqs;
pub mod utils;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }


}

