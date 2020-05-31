#[macro_use]
extern crate log;

use mysql::{prelude::*, Pool};
use std::collections::HashMap;
use tonic::{transport::Server, Request, Response, Status};

pub mod news {
    tonic::include_proto!("news");
}

pub struct News {
    pool: Pool,
}

impl News {
    fn map_category(req: news::get_meta_param::NewsCategory) -> String {
        match req {
            news::get_meta_param::NewsCategory::Hot => "news_hot",
            news::get_meta_param::NewsCategory::Society => "news_society",
            news::get_meta_param::NewsCategory::Entertainment => "news_entertainment",
            news::get_meta_param::NewsCategory::Tech => "news_tech",
            news::get_meta_param::NewsCategory::Military => "news_military",
            news::get_meta_param::NewsCategory::Sports => "news_sports",
            news::get_meta_param::NewsCategory::Car => "news_car",
            news::get_meta_param::NewsCategory::Finance => "news_finance",
            news::get_meta_param::NewsCategory::World => "news_world",
            news::get_meta_param::NewsCategory::Fashion => "news_fashion",
            news::get_meta_param::NewsCategory::Travel => "news_travel",
            news::get_meta_param::NewsCategory::Discovery => "news_discovery",
            news::get_meta_param::NewsCategory::Baby => "news_baby",
            news::get_meta_param::NewsCategory::Regimen => "news_regimen",
            news::get_meta_param::NewsCategory::Story => "news_story",
            news::get_meta_param::NewsCategory::Essay => "news_essay",
            news::get_meta_param::NewsCategory::Game => "news_game",
            news::get_meta_param::NewsCategory::History => "news_history",
            news::get_meta_param::NewsCategory::Food => "news_food",
        }
        .into()
    }
}

#[tonic::async_trait]
impl news::news_server::News for News {
    async fn get_meta(
        &self,
        request: Request<news::GetMetaParam>,
    ) -> Result<Response<news::GetMetaResult>, Status> {
        info!(
            "get_meta: request {:?} from {:?}",
            request.get_ref(),
            request.remote_addr(),
        );
        let request = request.into_inner();

        let mut conn = self
            .pool
            .get_conn()
            .map_err(|_| Status::resource_exhausted("can not connect to database"))?;

        let category = Self::map_category(unsafe { std::mem::transmute(request.category) });

        // id -> (title, behot_time, source, [image_list])
        let mut map = HashMap::new();

        conn.exec_map(
            "SELECT id, title, behot_time, source FROM news WHERE category = ?",
            (category,),
            |(id, title, behot_time, source): (String, String, i32, String)| {
                map.insert(
                    id.clone(),
                    news::get_meta_result::Info {
                        id,
                        title,
                        behot_time,
                        source,
                        image_list: vec![],
                    },
                );
            },
        )
        .map_err(|_| Status::resource_exhausted("database err"))?;

        for (k, v) in map.iter_mut() {
            let vec = conn
                .exec_map(
                    "SELECT url FROM news_image_list WHERE id = ?",
                    (&k,),
                    |url| url,
                )
                .map_err(|_| Status::resource_exhausted("database err"))?;

            v.image_list = vec;
        }

        Ok(Response::new(news::GetMetaResult {
            result: Some(news::get_meta_result::Result::Meta(
                news::get_meta_result::Meta {
                    info: map.into_iter().map(|(_, v)| v).collect(),
                },
            )),
        }))
    }

    async fn get_content(
        &self,
        request: Request<news::GetContentParam>,
    ) -> Result<Response<news::GetContentResult>, Status> {
        info!(
            "get_content: request {:?} from {:?}",
            request.get_ref(),
            request.remote_addr(),
        );
        let request = request.into_inner();

        let mut conn = self
            .pool
            .get_conn()
            .map_err(|_| Status::resource_exhausted("can not connect to database"))?;

        let content: Option<String> = conn
            .exec_first("SELECT content FROM news where id = ?", (request.id,))
            .map_err(|_| Status::resource_exhausted("database err"))?;

        if let Some(content) = content {
            Ok(Response::new(news::GetContentResult {
                result: Some(news::get_content_result::Result::Content(content)),
            }))
        } else {
            Ok(Response::new(news::GetContentResult {
                result: Some(news::get_content_result::Result::Error(news::Error {
                    code: 1,
                    message: "not found".into(),
                })),
            }))
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    info!("Connecting to database");
    let url = "mysql://root@mysql:3306/se";
    let pool = Pool::new(url)?;
    let news = News { pool };
    info!("Database connected");

    info!("Binding to 42222");
    let addr = "0.0.0.0:42222".parse().unwrap();
    Server::builder()
        .add_service(news::news_server::NewsServer::new(news))
        .serve(addr)
        .await?;

    Ok(())
}
