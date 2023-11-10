use axum::response::Html;
use tera::{Tera, Context};

static TEMPLATES_GLOB: &str = "templates/**/*";

#[derive(Clone)]
pub struct TemplateService {
  tera: Tera,
}

type TemplateResult<T> = Result<T, tera::Error>;

impl TemplateService {
  pub fn build() -> TemplateResult<Self> {
    let tera = Tera::new(TEMPLATES_GLOB)?;
    return Ok(Self { tera });
  }

  pub fn reload(&mut self) -> TemplateResult<()> {
    self.tera.full_reload()?;
    return Ok(());
  }

  pub fn render<T>(&self, name: &str, context: &T) -> TemplateResult<Html<String>>
  where
    T: serde::Serialize,
  {
    let context = &Context::from_serialize(context)?;
    let rendered = self.tera.render(name, context)?;
    return Ok(Html(rendered));
  }

  pub fn render_empty_context(&self, name: &str) -> TemplateResult<Html<String>> {
    let context = &Context::new();
    let rendered = self.tera.render(name, context)?;
    return Ok(Html(rendered));
  }
}