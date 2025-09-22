use dir_test::{Fixture, dir_test};
use f_ck::{DataWriter, JoinEngine, QueryPlan};
// use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[dir_test(
    dir: "$CARGO_MANIFEST_DIR/test_data/fixtures",
    glob: "*.json",
)]
fn test_fixture(fixture: Fixture<&str>) -> Result<(), anyhow::Error> {
    // tracing_subscriber::registry()
    //     .with(fmt::layer())
    //     .with(EnvFilter::from_default_env())
    //     .init();

    let plan = QueryPlan::from_json(fixture.content())?;

    let result = JoinEngine::execute_query(&plan)?;

    let preview_output = DataWriter::preview_data(result, None)?;

    insta::assert_snapshot!(preview_output);

    Ok(())
}
