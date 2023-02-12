use kube::api::{Patch, PatchParams};
use kube::{Api, Client, Error};
use serde_json::{json, Value};
use k8s_openapi::api::batch::v1::Job;


pub async fn add(client: Client, name: &str, namespace: &str) -> Result<Job, Error> {
    let api: Api<Job> = Api::namespaced(client, namespace);
    let finalizer: Value = json!({
        "metadata": {
            "finalizers": ["pushoperators.vitalbh.io/finalizer"]
        }
    });

    let patch: Patch<&Value> = Patch::Merge(&finalizer);
    Ok(api.patch(name, &PatchParams::default(), &patch).await?)
}


pub async fn delete(client: Client, name: &str, namespace: &str) -> Result<Job, Error> {
    let api: Api<Job> = Api::namespaced(client, namespace);
    let finalizer: Value = json!({
        "metadata": {
            "finalizers": null
        }
    });

    let patch: Patch<&Value> = Patch::Merge(&finalizer);
    Ok(api.patch(name, &PatchParams::default(), &patch).await?)
}
