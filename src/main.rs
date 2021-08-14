use actix_web::{error, get, post, web, App, Error, HttpResponse, HttpServer, Responder};
use chrono::prelude::{DateTime, FixedOffset};
use r2d2::Pool;
use r2d2_sqlite::{self, SqliteConnectionManager};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[get("/health")]
async fn health() -> impl Responder {
  HttpResponse::Ok().body("OK!!")
}

#[derive(Deserialize)]
struct GetTodoReqPath {
  id: String,
}

#[derive(Serialize, Deserialize)]
struct GetTodoRes {
  todos: Vec<Todo>,
}

#[get("/todo/{id}")]
async fn get_todo(
  db: web::Data<Pool<SqliteConnectionManager>>,
  req_path: web::Path<GetTodoReqPath>,
) -> Result<HttpResponse, Error> {
  println!("id: {}", req_path.id);
  let conn = db
    .get_ref()
    .get()
    .map_err(|e| error::ErrorInternalServerError(e))?;
  let mut stmt = conn
    .prepare("SELECT id, title, body FROM todo WHERE id = ?1")
    .map_err(|e| error::ErrorInternalServerError(e))?;
  let todo_itr = stmt
    .query_map(params![req_path.id], |row| {
      Ok(Todo {
        id: row.get(0)?,
        title: row.get(1)?,
        body: row.get(2)?,
        // created_at: DateTime::parse_from_rfc3339(&row.get(3)?)?,
      })
    })
    .map_err(|e| error::ErrorInternalServerError(e))?;

  let mut res = GetTodoRes { todos: Vec::new() };
  for todo in todo_itr {
    match todo {
      Ok(t) => res.todos.push(t),
      Err(e) => {
        println!("error todo {}", e);
        return Err(error::ErrorInternalServerError(e));
      }
    }
  }

  Ok(HttpResponse::Ok().json(res))
}

#[derive(Deserialize)]
struct PostTodoReq {
  title: String,
  body: Option<String>,
}

#[derive(Serialize)]
struct PostTodoRes {
  id: i64,
}

#[post("/todo")]
async fn post_todo(
  db: web::Data<Pool<SqliteConnectionManager>>,
  req: web::Json<PostTodoReq>,
) -> Result<HttpResponse, Error> {
  let conn = db
    .get_ref()
    .get()
    .map_err(|e| error::ErrorInternalServerError(e))?;

  let id = conn
    .execute(
      "INSERT INTO todo (title, body) VALUES(?1, ?2)",
      params![req.title, req.body],
    )
    .map(|_| conn.last_insert_rowid())
    .map_err(|e| error::ErrorInternalServerError(e))?;

  let res = PostTodoRes { id: id };

  Ok(HttpResponse::Ok().json(res))
}

#[derive(Serialize, Deserialize)]
struct Todo {
  id: i64,
  title: String,
  body: String,
  // created_at: DateTime<FixedOffset>,
}

fn init_db(pool: &Pool<SqliteConnectionManager>) -> Result<(), Box<dyn std::error::Error>> {
  let conn = pool.get()?;
  conn.execute(
    "CREATE TABLE todo (
      id          INTEGER PRIMARY KEY AUTOINCREMENT,
      title       TEXT,
      body        TEXT,
      created_at  TEXT
    )",
    params![],
  )?;

  Ok(())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
  let manager = SqliteConnectionManager::memory();
  let pool = Pool::new(manager).unwrap();
  // TODO: fix use manager
  match init_db(&pool) {
    Ok(_) => println!("succeed db init"),
    Err(err) => panic!("Error: {}", err),
  }
  HttpServer::new(move || {
    App::new()
      .data(pool.clone())
      .service(health)
      .service(get_todo)
      .service(post_todo)
  })
  .bind("127.0.0.1:8080")?
  .run()
  .await
}
