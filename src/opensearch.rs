use std::{env, collections::HashMap, vec::Vec};
use reqwest;
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use serde_json;
use crate::core::{GenericError};


// given a path, create the url from environment variables
fn path_url(path: &str) -> String {
    let host = match env::var("OPENSEARCH_HOST") {
        Ok(h) => h,
        Err(_) => String::from("http://localhost"),
    };
    let port = match env::var("OPENSEARCH_PORT") {
        Ok(p) => p,
        Err(_) => String::from("9200")
    };
    format!("{}:{}/{}", host, port, path)
}



/// get a path, returning a deserializable struct
pub async fn get<TS: DeserializeOwned> (path: &str) -> Result<TS, GenericError> {
    let url = path_url(path);
    let body: TS = reqwest::get(&url)
        .await?
        .json::<TS>()
        .await?;
    Ok(body)
}


pub enum Method {
    Get,
    Post,
    Put
}

/// make a request with a specified method and a serializable struct, expecting a deserializable struct bach 
pub async fn req_payload<TC: Serialize, TS: DeserializeOwned> (method: Method, path: &str, payload: &TC) -> Result<TS, GenericError> {
    let url = path_url(path);
    let client = reqwest::Client::new();
    let rb = match method {
        Method::Get => client.get(&url),
        Method::Post => client.post(&url),
        Method::Put => client.put(&url),
    };
    let body: TS = rb.json(&payload).send().await?.json::<TS>().await?;
    Ok(body)
}


#[derive(Deserialize, Debug)]
pub struct PingRespVersion {
    pub distribution: String,
    pub number: String,
    pub lucene_version: String,
    pub minimum_wire_compatibility_version: String,
    pub minimum_index_compatibility_version: String,
}

/// This is the struct that gets sent back when you ping a node/cluster
#[derive(Deserialize, Debug)]
pub struct PingResp {
    pub name: String,
    pub cluster_name: String,
    pub cluster_uuid: String,
    pub version: PingRespVersion,
    pub tagline: String,
}

/// Ping the node/cluster to ensure you can communicate
pub async fn ping() -> Result<PingResp, GenericError> {
    Ok(get("").await?)
}


/// This is the struct you pass when creating an index 
/// https://opensearch.org/docs/2.2/opensearch/supported-field-types/nested/
#[derive(Serialize)]
struct PutIndexReq {
    mappings: PutIndexMappings
}

#[derive(Serialize)]
struct PutIndexMappings {
    // an example might be Field-><type->nested>
    // see https://opensearch.org/docs/2.2/opensearch/supported-field-types/nested/
    properties: HashMap<String,HashMap<&'static str, serde_json::Value>>, 
}

// if you successfully create an index, this is what gets sent back 
#[derive(Deserialize, Debug)]
pub struct PutIndexRespSucc {
    pub acknowledged: bool,
    pub shards_acknowledged: bool,
    pub index: String,
}


#[derive(Deserialize, Debug)]
pub struct ErrRootCause {
    pub r#type: String,
    pub reason: String,
    pub index: String, 
    pub index_uuid: String,
}

#[derive(Deserialize, Debug)]
pub struct PutIndexErr {
    pub root_cause: Vec<ErrRootCause>,
    pub r#type: String,
    pub reason: String,
    pub index: String,
    pub index_uuid: String, 

}

/// This is the struct you get back if you try to create an index that already existed
#[derive(Deserialize, Debug)]
pub struct PutIndexRespErr {
    status: u16,
    error: PutIndexErr,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum PutIndexResp {
    Succ(PutIndexRespSucc),
    Error(PutIndexRespErr),
}

/// ensure an index exists
/// any fields supplied to nested_fields will have a nested index
pub async fn put_index(index: &str, nested_fields: &[&str]) -> Result<PutIndexResp, GenericError> {
    let mut properties = HashMap::new();
    for field in nested_fields {
        let mut field_params = HashMap::new();
        field_params.insert("type", serde_json::Value::String("nested".to_string()));
        properties.insert(field.to_string(), field_params);
    }
    let payload = PutIndexReq{mappings: PutIndexMappings{properties}};
    let resp: PutIndexResp = req_payload(Method::Put, index, &payload).await?;
    Ok(resp)
}


/// This represents how many shards a document is updated on when it is PUT
#[derive(Deserialize, Debug)]
pub struct DocShards {
    pub total: u8,
    pub successful: u8,
}

/// This is the response you get back when you PUT a document
#[derive(Deserialize, Debug)]
pub struct PutDocResp {
    pub _index: String,
    pub _id: String,
    pub _version: u16,
    pub result: String,
    pub _shards:DocShards,
}



/// use this method to place a document in an index
pub async fn put_doc<T: Serialize>(index: &str, _id: &str, doc: &T) -> Result<PutDocResp, GenericError> {
    let path = format!("{}/_doc/{}", index, _id);
    let resp: PutDocResp = req_payload(Method::Put, &path, doc).await?;
    Ok(resp)
}

/// This is the struct that gets sent back when you try to get a document by its id
#[derive(Deserialize)]
pub struct GetDocResp<T> {
    pub _index: String,
    pub _id: String,    // the _id you requested to get
    pub found: bool,    // true if the document was found
    pub _version: Option<u16>,  // increments each time you update the document
    pub _source: Option<T>,     // Some() variant if the document exists
}

/// get a document by its id
pub async fn get_doc<T: DeserializeOwned>(index: &str, _id: &str) -> Result<GetDocResp<T>, GenericError> {
    let path = format!("{}/_doc/{}", index, _id);
    Ok(get(&path).await?)
}


/// this struct captures how shards participated in a query
#[derive(Deserialize)]
pub struct QueryShards {
    pub failed: u8,
    pub skipped: u8,
    pub successful: u8,
    pub total: u8,
}

#[derive(Deserialize)]
pub struct QueryHit<T> {
    pub _id: String,
    pub _index: String,
    pub _score: f32,
    pub _source: T
}

#[derive(Deserialize)]
pub struct QueryHits<T> {
    pub hits: Vec<QueryHit<T>>,
    pub max_score: f32, 
}
/// This is the struct that gets sent back after making a query
#[derive(Deserialize)]
pub struct QueryResp<T> {
    pub _shards: QueryShards,
    pub hits: QueryHits<T>,
    pub timed_out: bool,
    pub took: u8,
}


/// query for documents by providing a query struct 
pub async fn query_payload<TQ: Serialize, T: DeserializeOwned>(index: &str, query: &TQ) -> Result<QueryResp<T>, GenericError> {
    let path = format!("{}/_search", index);
    Ok(req_payload(Method::Get, &path, query).await?)

}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;
    use serde_json::{json,Value};
    use tokio::runtime::Runtime;
    use chrono::NaiveDate;
    use rand::{distributions::Alphanumeric, Rng};

    const TEST_INDEX: &'static str = "test_idx";

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct DemoDoc {
        pub name: String,
        pub position: i32,
        pub date: NaiveDate,
    }
    
    #[test]
    fn test_ping() {
        // ensure you can ping the cluster
        let rt = Runtime::new().unwrap();
        rt.block_on(async{
            let _ping_resp = ping().await.unwrap();
            // no panic here means you successfully pinged the cluster/node
        })
    }

    #[test]
    fn test_put_index() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let resp = put_index(TEST_INDEX, &["some_field"]).await.unwrap();
            //println!("{:?}", &resp);
            match resp {
                PutIndexResp::Error(e) => println!("{:?}", &e),
                PutIndexResp::Succ(s) => println!("{:?}", &s),
            };
        })
    }

    #[test] 
    fn read_write_doc() {
        // ensure you can read and write a document
        let today = chrono::Utc::now().naive_local().date();
        let name: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(7).map(char::from).collect();
        let position = 72;
        let dd = DemoDoc{name, position, date: today};
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let resp = put_doc(TEST_INDEX, "123", &dd).await.unwrap();
            println!("{:?}", &resp);
            assert!(resp._shards.successful >= 1);
            let resp = get_doc(TEST_INDEX, "123").await.unwrap();
            println!("got this doc back: {:?}", &resp._source);
            assert_eq!(dd, resp._source.unwrap()); // ensure you got the same document back
        });
    }

    #[test]
    fn test_search_docs() {
        let today = chrono::Utc::now().naive_local().date();
        let dd1 = DemoDoc{name: "Tasty avocado".to_string(), position:-3, date: today};
        let dd2 = DemoDoc{name: "Tasty lemon".to_string(), position:123, date: today};
        let dd3 = DemoDoc{name: "Pretty peacock".to_string(), position:88, date: today};
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let _ = put_doc(TEST_INDEX, "doc1", &dd1).await.unwrap();
            let _ = put_doc(TEST_INDEX, "doc2", &dd2).await.unwrap();
            let _ = put_doc(TEST_INDEX, "doc3", &dd3).await.unwrap();
            let query = json!({"query":{"match":{"name":"avocado"}}});
            let resp: QueryResp<DemoDoc> = query_payload(TEST_INDEX, &query).await.unwrap();
            println!("query result 1 hits= {:?}", resp.hits.hits.get(0).unwrap()._source);
        });
        
    }
}
