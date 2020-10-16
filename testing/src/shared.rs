use actix::prelude::*;
use serde::{Deserialize, Serialize};

pub enum TestRequest {}

#[derive(MessageResponse, Serialize, Deserialize)]
pub enum TestResponse {}

impl Message for TestRequest {
    type Result = TestResponse;
}
