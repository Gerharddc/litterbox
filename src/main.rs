use futures::Future;
use russh::keys::*;

#[derive(Clone)]
struct X {}
impl agent::server::Agent for X {
    fn confirm(
        self,
        _: std::sync::Arc<PrivateKey>,
    ) -> Box<dyn Future<Output = (Self, bool)> + Send + Unpin> {
        println!("Confirm private key!");
        Box::new(futures::future::ready((self, true)))
    }

    async fn confirm_request(&self, msg: agent::server::MessageType) -> bool {
        print!("Confirm request: ");

        use agent::server::MessageType;
        match msg {
            MessageType::RequestKeys => {
                println!("RequestKeys");
            }
            MessageType::AddKeys => {
                println!("AddKeys");
            }
            MessageType::RemoveKeys => {
                println!("RemoveKeys");
            }
            MessageType::RemoveAllKeys => {
                println!("RemoveAllKeys");
            }
            MessageType::Sign => {
                println!("Sign");
            }
            MessageType::Lock => {
                println!("Lock");
            }
            MessageType::Unlock => {
                println!("Unlock");
            }
        }

        true
    }
}

const PKCS8_ENCRYPTED: &'static str = "-----BEGIN ENCRYPTED PRIVATE KEY-----\nMIIFLTBXBgkqhkiG9w0BBQ0wSjApBgkqhkiG9w0BBQwwHAQITo1O0b8YrS0CAggA\nMAwGCCqGSIb3DQIJBQAwHQYJYIZIAWUDBAEqBBBtLH4T1KOfo1GGr7salhR8BIIE\n0KN9ednYwcTGSX3hg7fROhTw7JAJ1D4IdT1fsoGeNu2BFuIgF3cthGHe6S5zceI2\nMpkfwvHbsOlDFWMUIAb/VY8/iYxhNmd5J6NStMYRC9NC0fVzOmrJqE1wITqxtORx\nIkzqkgFUbaaiFFQPepsh5CvQfAgGEWV329SsTOKIgyTj97RxfZIKA+TR5J5g2dJY\nj346SvHhSxJ4Jc0asccgMb0HGh9UUDzDSql0OIdbnZW5KzYJPOx+aDqnpbz7UzY/\nP8N0w/pEiGmkdkNyvGsdttcjFpOWlLnLDhtLx8dDwi/sbEYHtpMzsYC9jPn3hnds\nTcotqjoSZ31O6rJD4z18FOQb4iZs3MohwEdDd9XKblTfYKM62aQJWH6cVQcg+1C7\njX9l2wmyK26Tkkl5Qg/qSfzrCveke5muZgZkFwL0GCcgPJ8RixSB4GOdSMa/hAMU\nkvFAtoV2GluIgmSe1pG5cNMhurxM1dPPf4WnD+9hkFFSsMkTAuxDZIdDk3FA8zof\nYhv0ZTfvT6V+vgH3Hv7Tqcxomy5Qr3tj5vvAqqDU6k7fC4FvkxDh2mG5ovWvc4Nb\nXv8sed0LGpYitIOMldu6650LoZAqJVv5N4cAA2Edqldf7S2Iz1QnA/usXkQd4tLa\nZ80+sDNv9eCVkfaJ6kOVLk/ghLdXWJYRLenfQZtVUXrPkaPpNXgD0dlaTN8KuvML\nUw/UGa+4ybnPsdVflI0YkJKbxouhp4iB4S5ACAwqHVmsH5GRnujf10qLoS7RjDAl\no/wSHxdT9BECp7TT8ID65u2mlJvH13iJbktPczGXt07nBiBse6OxsClfBtHkRLzE\nQF6UMEXsJnIIMRfrZQnduC8FUOkfPOSXc8r9SeZ3GhfbV/DmWZvFPCpjzKYPsM5+\nN8Bw/iZ7NIH4xzNOgwdp5BzjH9hRtCt4sUKVVlWfEDtTnkHNOusQGKu7HkBF87YZ\nRN/Nd3gvHob668JOcGchcOzcsqsgzhGMD8+G9T9oZkFCYtwUXQU2XjMN0R4VtQgZ\nrAxWyQau9xXMGyDC67gQ5xSn+oqMK0HmoW8jh2LG/cUowHFAkUxdzGadnjGhMOI2\nzwNJPIjF93eDF/+zW5E1l0iGdiYyHkJbWSvcCuvTwma9FIDB45vOh5mSR+YjjSM5\nnq3THSWNi7Cxqz12Q1+i9pz92T2myYKBBtu1WDh+2KOn5DUkfEadY5SsIu/Rb7ub\n5FBihk2RN3y/iZk+36I69HgGg1OElYjps3D+A9AjVby10zxxLAz8U28YqJZm4wA/\nT0HLxBiVw+rsHmLP79KvsT2+b4Diqih+VTXouPWC/W+lELYKSlqnJCat77IxgM9e\nYIhzD47OgWl33GJ/R10+RDoDvY4koYE+V5NLglEhbwjloo9Ryv5ywBJNS7mfXMsK\n/uf+l2AscZTZ1mhtL38efTQCIRjyFHc3V31DI0UdETADi+/Omz+bXu0D5VvX+7c6\nb1iVZKpJw8KUjzeUV8yOZhvGu3LrQbhkTPVYL555iP1KN0Eya88ra+FUKMwLgjYr\nJkUx4iad4dTsGPodwEP/Y9oX/Qk3ZQr+REZ8lg6IBoKKqqrQeBJ9gkm1jfKE6Xkc\nCog3JMeTrb3LiPHgN6gU2P30MRp6L1j1J/MtlOAr5rux\n-----END ENCRYPTED PRIVATE KEY-----\n";

fn main() {
    env_logger::try_init().unwrap_or(());

    let dir = tempfile::tempdir().unwrap();
    let agent_path = dir.path().join("agent");
    println!("agent_path: {:#?}", agent_path);

    let core = tokio::runtime::Runtime::new().unwrap();
    let agent_path_ = agent_path.clone();

    // Starting a server
    let server_handle = core.spawn(async move {
        let listener = tokio::net::UnixListener::bind(&agent_path_).unwrap();
        russh::keys::agent::server::serve(
            tokio_stream::wrappers::UnixListenerStream::new(listener),
            X {},
        )
        .await
    });

    // Decode key using password
    let mut key = decode_secret_key(PKCS8_ENCRYPTED, Some("blabla")).unwrap();
    key.set_comment("Lekker key!");
    let public = key.public_key().clone();
    println!("Comment: {}", public.comment());

    core.block_on(async move {
        let stream = tokio::net::UnixStream::connect(&agent_path).await?;
        let mut client = agent::client::AgentClient::connect(stream);

        client
            .add_identity(
                &key,
                &[
                    //agent::Constraint::KeyLifetime { seconds: 60 },
                    agent::Constraint::Confirm,
                ],
            )
            .await?;

        let identities = client.request_identities().await?;
        println!("Identities: {:#?}", identities);

        let buf = b"signed message";
        let sig = client
            .sign_request(
                &public,
                None,
                russh_cryptovec::CryptoVec::from_slice(&buf[..]),
            )
            .await
            .unwrap();

        // Here, `sig` is encoded in a format usable internally by the SSH protocol.
        println!("sig: {:#?}", sig);

        Ok::<(), Error>(())
    })
    .unwrap();

    core.block_on(server_handle).unwrap().unwrap()
}
