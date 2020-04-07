#[macro_use] extern crate log;
use k8s_openapi::api::core::v1::Node;
use kube::{
    api::{Api, ListParams, Meta},
    runtime::Reflector,
    Client,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;

    let nodes: Api<Node> = Api::all(client.clone());
    let lp = ListParams::default()
        .labels("beta.kubernetes.io/instance-type=m4.2xlarge") // filter instances by label
        .timeout(10); // short watch timeout in this example
    let rf = Reflector::new(nodes, lp).init().await?;

    // rf is initialized with full state, which can be extracted on demand.
    // Output is an owned Vec<Node>
    rf.state().await?.into_iter().for_each(|o| {
        let labels = Meta::meta(&o).labels.clone().unwrap();
        info!(
            "Found node {} ({:?}) running {:?} with labels: {:?}",
            Meta::name(&o),
            o.spec.unwrap().provider_id.unwrap(),
            o.status.unwrap().conditions.unwrap(),
            labels
        );
    });

    let cloned = rf.clone();
    tokio::spawn(async move {
        loop {
            if let Err(e) = cloned.poll().await {
                warn!("Poll error: {:?}", e);
            }
        }
    });

    loop {
        // Update internal state by calling watch (waits the full timeout)
        rf.poll().await?;

        // Read the updated internal state (instant):
        let deploys: Vec<_> = rf.state().await?.iter().map(Meta::name).collect();
        info!("Current {} nodes: {:?}", deploys.len(), deploys);
    }
}