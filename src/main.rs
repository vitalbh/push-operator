mod finalizer;
use std::{sync::Arc};
use futures::{StreamExt};
use k8s_openapi::{api::{batch::v1::{Job, JobCondition}}, Metadata};
use kube::{
    api::{Api, ListParams},
    runtime::{Controller, controller::Action},
    Client, ResourceExt,
};

const HTTP_KEY: &str = "pushoperators.vitalbh.io/http";

enum JobAction {
    PushHttp(String),
    Free,
    Pending,
    NoOp,
}
struct ContextData {
    client: Client,
}

impl ContextData {
    pub fn new(client: Client) -> Self {
        ContextData { client }
    }
}

async fn watch_jobs(client: kube::Client, namespace: Option<String>, label: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    
    println!("namespace:{:?} label:{:?}", namespace.clone().unwrap_or_default().as_str(), label.clone().unwrap_or_default().as_str());

    let jobs: Api<Job> = Api::namespaced(client.clone(), namespace.unwrap_or_default().as_str());
    let lp = ListParams::default().labels(label.unwrap_or_default().as_str());
    let context: Arc<ContextData> = Arc::new(ContextData::new(client.clone()));

    Controller::new(jobs.clone(), lp)
        .run(reconciler, error_policy, context)
        .for_each(|reconciliation_result| async move {
            match reconciliation_result {
                Ok(job_resource) => {
                    println!("Reconciliation ok ->{:?}", job_resource);
                }
                Err(reconciliation_err) => {
                    eprintln!("Reconciliation error: {:?}", reconciliation_err)
                }
            }
        }).await;

    Ok(())
}

async fn reconciler(job: Arc<Job>, context: Arc<ContextData>) -> Result<Action, kube::Error> {
    let name = &job.name_any();
    let namespace = &job.namespace().unwrap_or_default();
    
    match determine_action(&job) {
        JobAction::PushHttp(path) => {
            match call(path).await {
                Ok(()) => println!("Pushed!!"),
                Err(e) => eprintln!("Oh noes, we don't know which era we're in! :( \n  {}", e),
            }
            finalizer::delete(context.client.clone(), name, namespace).await?;
        },
        JobAction::Free => {
            finalizer::delete(context.client.clone(), name, namespace).await?;
        }
        JobAction::Pending => {
            finalizer::add(context.client.clone(), name, namespace).await?;
        }
        JobAction::NoOp => {},
    };
    Ok(Action::await_change())
}

fn error_policy(job: Arc<Job>, _: &kube::Error, _: Arc<ContextData>) -> Action {
    println!("{:?} err job", job);
    Action::await_change()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;
    return watch_jobs(client, get_env("PUSH_NAMESPACE"), get_env("PUSH_LABEL")).await;
}


fn get_env(env: &str) -> Option<String> {
    let x = match std::env::var(env) {
        Ok(val) => Some(val),
        Err(_e) => None,
    };
    x
}


fn determine_action(job: &Job) -> JobAction {
    
    match job_has_completed(job) {
        None => {
            if job.metadata().deletion_timestamp.is_some() {
                return JobAction::Free;
            }
            JobAction::Pending
        },
        Some(_) => {
            match job
            .annotations()
            .get(HTTP_KEY) {
                None => JobAction::Free,
                Some(path) => {
                    if job
                        .metadata()
                        .finalizers
                        .as_ref()
                        .map_or(true, |finalizers| finalizers.is_empty()) {
                        return JobAction::NoOp;
                    }
                    JobAction::PushHttp(path.to_string())
                }
            }
        }
    }
}
fn job_has_completed(job: &Job) -> Option<&JobCondition> {
    let status = job.status.as_ref().unwrap();
    if let Some(conds) = &status.conditions {
        /*conds
        .iter()
        .filter(|c| c.type_ == "Complete" && c.status == "True")
        .for_each(|c| {println!("cond: {:?}",c); });
        println!("completed {:?}",completed);
        */

        return conds
            .iter()
            .filter(|c| c.type_ == "Complete" && c.status == "True")
            .last();
    }
    None
}

async fn call(path: String) -> Result<(), Box<dyn std::error::Error>> {
    println!("Sending push to: {:?}", path);
    reqwest::get(path).await?;
    Ok(())
}

