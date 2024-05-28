fn main() {
    embuild::espidf::sysenv::output();
    slint_build::compile_with_config(
        "ui/appwindow.slint",
        slint_build::CompilerConfiguration::new()
            // .with_style(String::from("material-light"))
            .embed_resources(slint_build::EmbedResourcesKind::EmbedForSoftwareRenderer),
    )
    .unwrap();
}
