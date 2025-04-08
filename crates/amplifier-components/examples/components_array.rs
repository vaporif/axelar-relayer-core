use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use relayer_amplifier_api_integration::Config;
use relayer_amplifier_api_integration::amplifier_api::identity::Identity;
use supervisor::{Worker, WorkerBuildFn};
use url::Url;

fn get_config() -> eyre::Result<Config> {
    let cfg = Config::builder()
        .url(Url::from_str("https://amplifier-devnet-amplifier.com").expect("works"))
        .identity(Identity::new(
            reqwest::Identity::from_pem(PEM.as_bytes()).expect("works"),
        ))
        .chain("blochain_name_on_axelar".to_string())
        .build();
    Ok(cfg)
}

#[cfg(feature = "nats")]
fn get_nats_urls() -> eyre::Result<Vec<Url>> {
    Ok(vec!["nats://localhost:4222"])
}

#[tokio::main]
async fn main() {
    amplifier_components::register_backtrace();
    let shutdown = amplifier_components::register_ctrlc_handler();

    let mut components: HashMap<String, WorkerBuildFn> = HashMap::new();

    #[cfg(feature = "nats")]
    {
        use amplifier_components::nats::{new_amplifier_ingester, new_amplifier_subscriber};

        tracing::debug!("amplifier ingester enabled");
        components.insert(
            "[amplifier-ingester]".to_string(),
            Box::new(|| {
                Box::pin(async {
                    let component = new_amplifier_ingester(get_config, get_nats_urls).await?;
                    Ok(Box::new(component) as Box<dyn Worker>)
                })
            }),
        );

        tracing::debug!("amplifier subscriber enabled");
        components.insert(
            "[amplifier-subscriber]".to_string(),
            Box::new(|| {
                Box::pin(async {
                    let component = new_amplifier_subscriber(get_config, get_nats_urls).await?;
                    Ok(Box::new(component) as Box<dyn Worker>)
                })
            }),
        );
    }

    println!("press ctrl+c to stop example");
    supervisor::run(components, &shutdown, Duration::from_secs(5)).expect("starts");
}

const PEM: &str = "-----BEGIN RSA PRIVATE KEY-----
MIIEpAIBAAKCAQEA5EZibAiV9ZLx5yYg5nsZhukPt+9zvK4rJbSeu4mQN538KMl1
MiKxFrxsLVW5idGuWrhOkYw5EBdbHDzmOnRsVLepv7axqc/+P40swGYFzZZwkoaZ
OpK/vGr4UEjlnGSVwDM6iKp3EjoH+Z0MSVUNNvAoW+5ynIZRJxCH8pumUHbmOsIZ
mBWynuIEpeQsX0a413sHhtF4dKKeReOnIvUCwU/zwXHvl8D+UdZAkpCcuLKxKh4h
iN534SF3t/mAyb2PdX61HQurDRcFES3zIGO5ExU5mV8bQt1skM7fBfIEkp9cNC5s
hNUwItN3DVMNBOrnf2pkrkpKeWzM3kgEk6fHawIDAQABAoIBACwErIbzkuMzbkUp
848uLqp2t6q62GEKXtSbAz7blH09EDpXOqurx+U/+UY2sRvz3ICu6KulU/2X22BT
F/wuphMiBXAsDQ9XRcpAcWA6bCUMPPHsVZDXanStVeu5WtlxBfV1i3R+Fo7jtNT7
5Tog1fcBkW7EyMIJHo3/YI+2VvhNz883qKHF+JEfrbc+EBttZAFIZvB2ml08Mz+z
StN9OWuk/OprAhDtuUNVMq2I766np5E13j3Y1MOcx6uvqUpgie32RxAGcb24NhVb
vNCWrn7srAuecDL0sglxDQ5uOvU4bbVw9/LrfdwBMX8/BSDIb5hPt9z5l6CXo851
/P/nwxkCgYEA/83a1nJ90GK47DnpQsvSkNKMmq8stPlUwJaG6Kf+0x9Wfb1sRVsb
wp+t6xZzfd9kWh1XBkLhsBfpSFS/WSeNrpg5A5hJ3cuZmQN7Q4kQopzU104Z6u96
zHpF8mD756Ryh2RF1iCpQrLJmxSTxULY4OMUsnuS/QL/giZC7o83RVMCgYEA5HMi
EmjTI6Ij6a4S2WvM5OyLKRhQUzg90pYKPG8Hd56FwDWiLBYua8ZyJp15maf2fYVh
ULKDhMv0C1xkzogW0GUu/BGn2NHAmU5+gpK0vFiCtOBp/VcvhEMUHyAo4TaWMLYg
sSINqvGNOhujyzsqntkzgjHgUj/rakHX+/Vp2okCgYEA27sROto4FpNms4v/QaBh
bINfwdOtfHscAR6MHjeIXgPyQKpA4cakLucI9wJfDIWi5wGC6l7zDFdNzRL3Fvcx
7gLWHq9m6/1jIBvsexO21WgQMC3nd3ZkrlZt1QjX+Z+5vXE1x/xgSGnZYbwoPu7v
6yVEdxhNJ8a2gi6pmdAtsv8CgYEAodNM122Z4yv/9JBymcFbKu7ExR+MBudYI8xV
A+pCh+GrLTTQ5BnyWDYCtofmk4n/eXr6LIfH0lIHVeTlI8gTIRwL5FM9asFqhS6t
PyFdFe486JPvgJ458p7xqfrF+oMIcQkSr2dH90eUmwzpQhVvY4bIjfl2xcyxnlt1
++0kRsECgYB5EXtzwqgEZdF01d/S5sHB9QJw5zNRfJpAsqmHK8wwLwBgnHL7Nk4d
oAVwJ70zDj0qhsPuHyxVCgrzwxOY1Jta3HdDK+WsV6W0DQyjTYUFOolW+QkYrkO1
IUgTbe3iyKoYM8q906OQL99TIZ73ZbhPNpWdAn6s1fmxYJcrhN/FNw==";
