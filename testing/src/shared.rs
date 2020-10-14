use actix::prelude::*;
use serde::{Serialize, Deserialize};

pub enum TestRequest {

}

#[derive(MessageResponse, Serialize, Deserialize)]
pub enum TestResponse {

}

impl Message for TestRequest {
    type Result = TestResponse;
}
