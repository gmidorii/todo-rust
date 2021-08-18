use chrono::prelude::{DateTime, FixedOffset, Local};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Todo {
  pub id: i64,
  pub title: String,
  pub body: String,
  pub created_at: Option<DateTime<FixedOffset>>,
}

#[derive(Deserialize)]
pub struct GetTodoReqPath {
  pub id: String,
}

#[derive(Serialize, Deserialize)]
pub struct GetTodoRes {
  pub todo: Todo,
}
