use actix_web::{error, get, web, App, Error, HttpResponse, HttpServer, Responder};
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
  let conn = match db.get_ref().get() {
    Ok(conn) => conn,
    Err(e) => return Err(error::ErrorInternalServerError(e)),
  };
  let mut stmt = match conn.prepare("SELECT id, title, body FROM todo WHERE id = ?1") {
    Ok(stmt) => stmt,
    Err(e) => return Err(error::ErrorInternalServerError(e)),
  };
  let todo_itr = match stmt.query_map(params![req_path.id], |row| {
    Ok(Todo {
      id: row.get(0)?,
      title: row.get(1)?,
      body: row.get(2)?,
      // created_at: DateTime::parse_from_rfc3339(&row.get(3)?)?,
    })
  }) {
    Ok(itr) => itr,
    Err(e) => return Err(error::ErrorInternalServerError(e)),
  };

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

#[derive(Serialize, Deserialize)]
struct Todo {
  id: i32,
  title: String,
  body: String,
  // created_at: DateTime<FixedOffset>,
}

fn init_db(pool: &Pool<SqliteConnectionManager>) -> Result<(), Box<dyn std::error::Error>> {
  let conn = match pool.get() {
    Ok(conn) => conn,
    Err(e) => return Err(Box::new(e)),
  };
  conn.execute(
    "CREATE TABLE todo (
      id          INTEGER PRIMARY KEY AUTOINCREMENT,
      title       TEXT,
      body        TEXT,
      created_at  TEXT
    )",
    params![],
  )?;

  // insert sample data
  conn.execute(
    "INSERT INTO todo (title, body) VALUES(?1, ?2)",
    params!["title", "body"],
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
  })
  .bind("127.0.0.1:8080")?
  .run()
  .await
}
