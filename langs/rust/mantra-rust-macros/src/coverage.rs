pub struct ReqCov {
    pub id: &'static str,
    pub file: &'static str,
    pub line: u32,
}

#[cfg(feature = "defmt")]
impl defmt::Format for ReqCov {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(
            fmt,
            "mantra: req-id='{=str}'; file='{=str}'; line='{}';",
            self.id,
            self.file,
            self.line
        )
    }
}

impl core::fmt::Display for ReqCov {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "mantra: req-id='{}'; file='{}'; line='{}';",
            self.id, self.file, self.line
        )
    }
}
