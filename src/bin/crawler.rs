#[macro_use]
extern crate log;

use futures::future::join_all;
use mysql::{params, prelude::*, Pool};

#[derive(Debug)]
struct News {
    id: String,
    category: String,
    title: String,
    content: String,
    behot_time: i32,
    source: String,
    image_list: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    info!("Connecting to database");
    let url = "mysql://root@mysql:3306/se";
    let pool = Pool::new(url)?;
    let mut conn = pool.get_conn()?;
    info!("Database connected");

    info!("Creating tables");
    conn.query_drop(
        r"
        CREATE TABLE IF NOT EXISTS news (
            id VARCHAR(30) PRIMARY KEY,
            category VARCHAR(16) NOT NULL,
            title VARCHAR(255) NOT NULL,
            content VARCHAR(4000) NOT NULL,
            behot_time INT NOT NULL,
            source VARCHAR(30) NOT NULL
        )
    ",
    )?;
    conn.query_drop(
        r"
        CREATE TABLE IF NOT EXISTS news_image_list (
            id VARCHAR(30) NOT NULL,
            url VARCHAR(255) NOT NULL
        )
    ",
    )?;
    info!("Tables created");

    info!("Creating index");
    let _ = conn.query_drop(
        r"
        ALTER TABLE news ADD INDEX category_index (category);
    ",
    ); // ignore if exists
    info!("Index created");

    let categories = vec![
        "news_hot",
        "news_society",
        "news_entertainment",
        "news_tech",
        "news_military",
        "news_sports",
        "news_car",
        "news_finance",
        "news_world",
        "news_fashion",
        "news_travel",
        "news_discovery",
        "news_baby",
        "news_regimen",
        "news_story",
        "news_essay",
        "news_game",
        "news_history",
        "news_food",
    ];

    info!("Fetching news");
    let res = join_all(categories
        .into_iter()
        .map(|category| async move {
            let url = format!("https://www.toutiao.com/api/article/feed/?category={}&_signature=6PtN5AAAtjQXBLIbWEaAHOj7Tf", category);

            let resp: serde_json::Value = reqwest::Client::new()
                .get(&url)
                .header("user-agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/83.0.4103.61 Safari/537.36")
                .send()
                .await?
                .json()
                .await?;

            let data = resp
                .get("data")
                .ok_or("no data")?
                .as_array()
                .ok_or("wrong type")?;

            let mut res = Vec::with_capacity(data.len());
            for obj in data {
                let f = || async move {
                    let title = obj
                        .get("title")
                        .ok_or("no title")?
                        .as_str()
                        .ok_or("wrong type")?
                        .to_owned();

                    let id = obj
                        .get("item_id")
                        .ok_or("no item id")?
                        .as_str()
                        .ok_or("wrong type")?
                        .to_owned();

                    let behot_time = obj
                        .get("behot_time")
                        .ok_or("no behot time")?
                        .as_i64()
                        .ok_or("wrong type")?
                        .to_owned();

                    let source = obj
                        .get("source")
                        .ok_or("no source")?
                        .as_str()
                        .ok_or("wrong type")?
                        .to_owned();

                    let mut image_list = obj
                        .get("image_list")
                        .and_then(|l| l.as_array())
                        .unwrap_or(&vec![])
                        .into_iter()
                        .map(|v| v.as_str().map(|v| format!("https:{}", v)).ok_or("wrong type"))
                        .collect::<Result<Vec<_>, _>>()?;

                    if image_list.is_empty() {
                        if let Some(url) = obj.get("image_url") {
                            if let Some(url) = url.as_str() {
                                image_list.push(format!("https:{}", url));
                            }
                        }
                    }

                    let content_url = format!(
                        "http://m.toutiao.com/i{}/info/",
                        id
                    );

                    let resp = reqwest::Client::new()
                        .get(&content_url)
                        .header("user-agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/83.0.4103.61 Safari/537.36")
                        .send()
                        .await?;

                    let resp: serde_json::Value = resp
                        .json()
                        .await?;

                    let content = resp.get("data")
                        .ok_or("no data")?
                        .get("content")
                        .ok_or("no content")?
                        .as_str()
                        .ok_or("wrong type")?.to_owned();

                    let res: Result<_, Box<dyn std::error::Error>> = Ok(News {
                        id,
                        category: category.to_owned(),
                        title,
                        content,
                        behot_time: behot_time as i32,
                        source,
                        image_list,
                    });

                    res
                };

                let r = f().await;

                if let Ok(r) = r {
                    res.push(r);
                }
            }
            let res: Result<_, Box<dyn std::error::Error>> = Ok(res);

            res
    })).await;

    for v in res {
        if let Ok(v) = v {
            info!("got {}", v.len());

            conn.exec_batch(
                r"INSERT IGNORE INTO news (id, category, title, content, behot_time, source)
                 VALUES (:id, :category, :title, :content, :behot_time, :source)",
                v.iter().map(|p| {
                    params! {
                        "id" => &p.id,
                        "category" => &p.category,
                        "title" => &p.title,
                        "content" => &p.content,
                        "behot_time" => &p.behot_time,
                        "source" => &p.source,
                    }
                }),
            )?;

            conn.exec_batch(
                r"INSERT IGNORE INTO news_image_list (id, url)
                 VALUES (:id, :url)",
                v.iter().flat_map(|p| {
                    p.image_list.iter().map(move |image| {
                        params! {
                            "id" => &p.id,
                            "url" => &image,
                        }
                    })
                }),
            )?;
        }
    }

    Ok(())
}
