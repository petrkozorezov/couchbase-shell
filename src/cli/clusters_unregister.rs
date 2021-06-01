use crate::state::State;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_stream::OutputStream;
use std::sync::{Arc, Mutex};

pub struct ClustersUnregister {
    state: Arc<Mutex<State>>,
}

impl ClustersUnregister {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl nu_engine::WholeStreamCommand for ClustersUnregister {
    fn name(&self) -> &str {
        "clusters unregister"
    }

    fn signature(&self) -> Signature {
        Signature::build("clusters unregister").required(
            "identifier",
            SyntaxShape::String,
            "the identifier to use for this cluster",
        )
    }

    fn usage(&self) -> &str {
        "Registers a cluster for use with the shell"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        clusters_unregister(args, self.state.clone())
    }
}

fn clusters_unregister(
    args: CommandArgs,
    state: Arc<Mutex<State>>,
) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once()?;

    let identifier = match args.nth(0) {
        Some(v) => match v.as_string() {
            Ok(name) => name,
            Err(e) => return Err(e),
        },
        None => return Err(ShellError::unexpected("identifier is required")),
    };

    let mut guard = state.lock().unwrap();
    if guard.remove_cluster(identifier).is_none() {
        return Err(ShellError::unexpected(
            "identifier is not registered to a cluster",
        ));
    };

    Ok(OutputStream::empty())
}