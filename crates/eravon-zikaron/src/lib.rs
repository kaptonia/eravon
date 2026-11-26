#![forbid(unsafe_code)]

pub mod discover;
pub mod fault;
pub mod index;
pub mod pack;
pub mod seam;

pub mod mark {
    pub const FEATURE: eravon_literals::trace::Feature = eravon_literals::trace::Feature::A4;

    #[inline]
    pub fn mark(feature: eravon_literals::trace::Feature) {
        eravon_literals::trace::mark(feature);
    }
}

#[derive(Clone, Debug)]
pub struct Survey {
    pub link: discover::Link,
    pub kits: Vec<index::Kit>,
    pub absent: Option<fault::Absent>,
}

pub fn survey(home: &str, manual: Option<&str>) -> Result<Survey, fault::Fault> {
    mark::mark(mark::FEATURE);
    let link = discover::discover(home, manual)?;
    match index::load(&link.at) {
        Ok(ix) => Ok(Survey { link, kits: ix.kits, absent: None }),
        Err(e) => match e.absent() {
            Some(a) => Ok(Survey { link, kits: Vec::new(), absent: Some(a.clone()) }),
            None => Err(e),
        },
    }
}
