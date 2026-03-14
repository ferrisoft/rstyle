fn init(app: u32) {
    web_document
        .with_element_by_id("gl-root", |root| {
            let root_html = root.dyn_as_html_element()
                .expect("msg");
            app.mount_to(root_html).warn();
        })
        .warn();
}
