//! Generated types support documentation
#![deny(missing_docs)]

mod finalizer;
use std::{collections::BTreeMap, sync::Arc};
use futures::StreamExt;
use k8s_openapi::{api::{batch::v1::Job}, Metadata};
use kube::{
    api::{Api, ListParams},
    runtime::{Controller, controller::Action, reflector::ObjectRef},
    Client, ResourceExt,
};

const HTTP_KEY: &str = "pushoperators.vitalbh.io/http";

fn has_http_annotation(meta: &BTreeMap<String,String>) -> bool {
    meta.contains_key(HTTP_KEY)
}

enum JobAction {
    PushHttp(String),
    Free,
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

async fn watch_jobs(client: kube::Client) -> Result<(), Box<dyn std::error::Error>> {
   
    async fn reconciler(job: Arc<Job>, context: Arc<ContextData>) -> Result<Action, kube::Error> {
        let name = &job.name_any();
        let namespace = &job.namespace().unwrap_or_default();
        return match determine_action(&job) {
            JobAction::PushHttp(path) => {

               if job
                .metadata()
                .finalizers
                .as_ref()
                .map_or(true, |finalizers| finalizers.is_empty()) {
                    return Ok(Action::await_change())
                }
                //TODO
                println!("Sending push to: {:?}", path);

                finalizer::delete(context.client.clone(), name, namespace).await?;
                Ok(Action::await_change())
            },
            JobAction::Free => {
                finalizer::delete(context.client.clone(), name, namespace).await?;
                Ok(Action::await_change())
            }
            JobAction::NoOp => {
                finalizer::add(context.client.clone(), name, namespace).await?;
                Ok(Action::await_change())
            },
        };

        
        //Ok(Action::await_change())
    }
    
    fn error_policy(job: Arc<Job>, _: &kube::Error, _: Arc<ContextData>) -> Action {
        println!("{:?} err job", job);
        Action::await_change()
    }
    //let crd_api: Api<PushOperator> = Api::all(client.clone());
    let jobs: Api<Job> = Api::namespaced(client.clone(), "vitalns");
    let wjobs: Api<Job> = Api::namespaced(client.clone(), "vitalns");
    let context: Arc<ContextData> = Arc::new(ContextData::new(client.clone()));
    Controller::new(jobs, ListParams::default()).watches(
        wjobs,
        ListParams::default(),
        |job| {
            match job.annotations().get(HTTP_KEY) {
                
                Some(_) => Some(ObjectRef::from_obj(&job)),
                None =>  None
            }
        }

    ).run(reconciler, error_policy, context)
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
   
    let client = Client::try_default().await?;
    return watch_jobs(client).await;
}



fn determine_action(job: &Job) -> JobAction {
    let completed = job_has_completed(job);
    
    if job.metadata().deletion_timestamp.is_some() 
        && !completed {
        return JobAction::Free
    }
    
    if !completed {
        return JobAction::NoOp
    }
    
    let filtered_jobs = job
        .metadata()
        .annotations
        .clone()
        .filter(has_http_annotation);

    match filtered_jobs {
        None => JobAction::Free,
        Some(annotation) => {
            match annotation.get(HTTP_KEY) {
                Some(path) => {JobAction::PushHttp(path.to_string())},
                None => {JobAction::Free}
            }
        }
    }
   
}
fn job_has_completed(job: &Job) -> bool {
    let status = job.status.as_ref().unwrap();
    if let Some(conds) = &status.conditions {
        let completed = conds
            .iter()
            .filter(|c| c.type_ == "Complete" && c.status == "True")
            .last();
            
            /*conds
            .iter()
            .filter(|c| c.type_ == "Complete" && c.status == "True")
            .for_each(|c| {println!("cond: {:?}",c); });
            println!("completed {:?}",completed);
            */
            
        match completed {
            None => return false,
            Some(_) => return true
        }
    }
    false
}


