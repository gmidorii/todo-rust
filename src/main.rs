use actix_web::{error, get, post, web, App, Error, HttpResponse, HttpServer, Responder};
use chrono::prelude::{DateTime, FixedOffset, Local};
use r2d2::Pool;
use r2d2_sqlite::{self, SqliteConnectionManager};
use rusqlite::params;
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
  todo: Todo,
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
    .prepare("SELECT id, title, body, created_at FROM todo WHERE id = ?1")
    .map_err(|e| error::ErrorInternalServerError(e))?;
  let todo_itr = stmt
    .query_map(params![req_path.id], |row| {
      let ss: Option<String> = match row.get(3) {
        Ok(s) => s,
        Err(_) => None,
      };
      let created_at: Option<DateTime<FixedOffset>> = match ss {
        Some(s) => Some(DateTime::parse_from_rfc3339(&s).map_err(|e| {
          rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(e))
        })?),
        None => None,
      };
      Ok(Todo {
        id: row.get(0)?,
        title: row.get(1)?,
        body: row.get(2)?,
        created_at: created_at,
      })
    })
    .map_err(|e| error::ErrorInternalServerError(e))?;

  for todo in todo_itr {
    match todo {
      Ok(t) => {
        let res = GetTodoRes { todo: t };
        return Ok(HttpResponse::Ok().json(res));
      }
      Err(e) => {
        println!("error todo {}", e);
        return Err(error::ErrorInternalServerError(e));
      }
    }
  }

  Err(error::ErrorNotFound("not found todo"))
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

  let now = Local::now();
  let id = conn
    .execute(
      "INSERT INTO todo (title, body, created_at) VALUES(?1, ?2, ?3)",
      params![req.title, req.body, now.to_rfc3339()],
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
  created_at: Option<DateTime<FixedOffset>>,
}

fn init_db(pool: &Pool<SqliteConnectionManager>) -> Result<(), Box<dyn std::error::Error>> {
  let conn = pool.get()?;
  conn.execute(
    "CREATE TABLE IF NOT EXISTS todo (
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
  let manager = SqliteConnectionManager::file("./todo.sqlite3");
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
