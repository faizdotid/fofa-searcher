pub mod fofa_response {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    #[allow(unused)]
    pub struct SearchResponse {
        pub error: bool,
        pub consumed_fpoint: u8,
        pub required_fpoints: u8,
        pub tip: String,
        pub size: u32,
        pub page: u32,
        pub mode: String,
        pub query: String,
        pub results: Vec<[String; 3]>,
    }

    #[derive(Debug, Deserialize)]
    #[allow(unused)]
    pub struct ErrorResponse {
        pub error: bool,
        pub errmsg: String,
    }
    
}


