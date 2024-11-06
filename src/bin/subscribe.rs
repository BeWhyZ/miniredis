use futures::StreamExt;
use mini_redis::{self, client};



async fn publish() -> mini_redis::Result<()> {
    let mut client = client::connect("127.0.0.1:6379").await?;

    client.publish("numbers", "1".into()).await?;
    client.publish("numbers", "2".into()).await?;
    client.publish("numbers", "two".into()).await?;
    client.publish("numbers", "three".into()).await?;
    client.publish("numbers", "4".into()).await?;
    Ok(())
}


async fn subscribe() -> mini_redis::Result<()> {
    let client = client::connect("127.0.0.1:6379").await?;
    let subscriber = client.subscribe(vec!["numbers".to_string()]).await?;
    let message = subscriber.into_stream();

    tokio::pin!(message);

    while let Some(msg) = message.next().await {
        println!("got {:?}", msg)
    }

    Ok(())
}


#[tokio::main]
async fn main() -> mini_redis::Result<()>{
    tokio::spawn(async {
        publish().await
    });

    subscribe().await?;
    println!("DONE");

    Ok(())
}

