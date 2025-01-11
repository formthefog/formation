use form_broker::{broker::Broker, publisher::{Publisher, PubStream}, subscriber::{Subscriber, SubStream}};
use tokio::task;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let broker = Broker::new("0.0.0.0:5555", "0.0.0.0:5556").await?;
    tokio::spawn(async move {
        broker.start().await.expect("Broker failed");
    });

    let mut subscriber_1 = Subscriber::new("0.0.0.0:5556", vec!["hello".to_string()]).await?;
    task::spawn(async move {
        loop {
            tokio::select! {
                result = subscriber_1.receive() => {
                    match result {
                        Ok(messages) => {
                            for message in messages {
                                log::info!("Subscriber-1 Received: {:?}", String::from_utf8_lossy(&message));
                            }
                        }
                        Err(e) => {
                            log::error!("Error receiving message: {}", e);
                            break;
                        }
                    }
                },
                _ = tokio::signal::ctrl_c() => {
                    log::info!("Received kill signal");
                    break;
                }
            }
        }
    });


    let mut subscriber_2 = Subscriber::new("0.0.0.0:5556", vec!["goodbye".to_string()]).await?;
    task::spawn(async move {
        loop {
            tokio::select! {
                result = subscriber_2.receive() => {
                    match result {
                        Ok(messages) => {
                            for message in messages {
                                log::info!("Subscriber-2 Received: {:?}", String::from_utf8_lossy(&message));
                            }
                        }
                        Err(e) => {
                            log::error!("Error receiving message: {}", e);
                            break;
                        }
                    }
                },
                _ = tokio::signal::ctrl_c() => {
                    log::error!("Received kill signal");
                    break;
                }
            }
        }
    });


    let mut publisher = Publisher::new("0.0.0.0:5555").await?;
    for i in 0..10 {
        let message = format!("Hello World {}", i);
        let topic = "hello";
        publisher.publish(topic.to_string(), &message).await.expect("unable to write message to publisher_stream");

        let message = format!("Goodbye World {}", i);
        let topic = "goodbye";
        publisher.publish(topic.to_string(), &message).await.expect("Unable to write message to publisher_stream");
    }

    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    Ok(())
}
