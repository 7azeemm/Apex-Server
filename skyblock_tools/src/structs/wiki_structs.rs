use derive_new::new;
use getset::Getters;

#[derive(Debug, Clone, new, Getters)]
#[getset(get = "pub")]
pub struct WikiPage {
    title: String,
    introduction: Option<String>,
    sections: Vec<Section>,
}

#[derive(Debug, Clone, new, Getters)]
#[getset(get = "pub")]
pub struct Section {
    title: String,
    content: String,
}
