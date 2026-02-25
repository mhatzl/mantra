use mantra_schema::annotations::TraceKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeTraceVariant {
    Req,
    ReqImpl,
    ReqSatisfied,
    ReqVerified,
    ReqTest,
    ReqNote,
    ReqLink,
}

impl AttributeTraceVariant {
    pub fn as_str(&self) -> &'static str {
        match self {
            AttributeTraceVariant::Req => "req",
            AttributeTraceVariant::ReqImpl => "req_impl",
            AttributeTraceVariant::ReqSatisfied => "req_satisfied",
            AttributeTraceVariant::ReqVerified => "req_verified",
            AttributeTraceVariant::ReqTest => "req_test",
            AttributeTraceVariant::ReqNote => "req_note",
            AttributeTraceVariant::ReqLink => "req_link",
        }
    }
}

impl std::str::FromStr for AttributeTraceVariant {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            _ if s == AttributeTraceVariant::Req.as_str() => Ok(AttributeTraceVariant::Req),
            _ if s == AttributeTraceVariant::ReqImpl.as_str() => Ok(AttributeTraceVariant::ReqImpl),
            _ if s == AttributeTraceVariant::ReqSatisfied.as_str() => {
                Ok(AttributeTraceVariant::ReqSatisfied)
            }
            _ if s == AttributeTraceVariant::ReqVerified.as_str() => {
                Ok(AttributeTraceVariant::ReqVerified)
            }
            _ if s == AttributeTraceVariant::ReqTest.as_str() => Ok(AttributeTraceVariant::ReqTest),
            _ if s == AttributeTraceVariant::ReqNote.as_str() => Ok(AttributeTraceVariant::ReqNote),
            _ if s == AttributeTraceVariant::ReqLink.as_str() => Ok(AttributeTraceVariant::ReqLink),
            _ => Err(anyhow::anyhow!("Invalid attribute trace variant: {}", s)),
        }
    }
}

impl From<AttributeTraceVariant> for TraceKind {
    fn from(value: AttributeTraceVariant) -> Self {
        match value {
            AttributeTraceVariant::Req
            | AttributeTraceVariant::ReqImpl
            | AttributeTraceVariant::ReqSatisfied => TraceKind::Satisfies,
            AttributeTraceVariant::ReqVerified | AttributeTraceVariant::ReqTest => {
                TraceKind::Verifies
            }
            AttributeTraceVariant::ReqNote => TraceKind::Clarifies,
            AttributeTraceVariant::ReqLink => TraceKind::Links,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FnLikeTraceVariant {
    SatisfyReq,
    ImplReq,
    VerifyReq,
    AssertReq,
    AssertEqReq,
    AssertNeReq,
    DebugAssertReq,
    DebugAssertEqReq,
    DebugAssertNeReq,
    ClarifyReq,
    LinkReq,
}

impl FnLikeTraceVariant {
    pub fn as_str(&self) -> &'static str {
        match self {
            FnLikeTraceVariant::SatisfyReq => "satisfy_req",
            FnLikeTraceVariant::ImplReq => "impl_req",
            FnLikeTraceVariant::VerifyReq => "verify_req",
            FnLikeTraceVariant::AssertReq => "assert_req",
            FnLikeTraceVariant::AssertEqReq => "assert_eq_req",
            FnLikeTraceVariant::AssertNeReq => "assert_ne_req",
            FnLikeTraceVariant::DebugAssertReq => "debug_assert_req",
            FnLikeTraceVariant::DebugAssertEqReq => "debug_assert_eq_req",
            FnLikeTraceVariant::DebugAssertNeReq => "debug_assert_ne_req",
            FnLikeTraceVariant::ClarifyReq => "clarify_req",
            FnLikeTraceVariant::LinkReq => "link_req",
        }
    }
}

impl std::str::FromStr for FnLikeTraceVariant {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            _ if s == FnLikeTraceVariant::SatisfyReq.as_str() => Ok(FnLikeTraceVariant::SatisfyReq),
            _ if s == FnLikeTraceVariant::ImplReq.as_str() => Ok(FnLikeTraceVariant::ImplReq),
            _ if s == FnLikeTraceVariant::VerifyReq.as_str() => Ok(FnLikeTraceVariant::VerifyReq),
            _ if s == FnLikeTraceVariant::AssertReq.as_str() => Ok(FnLikeTraceVariant::AssertReq),
            _ if s == FnLikeTraceVariant::AssertEqReq.as_str() => {
                Ok(FnLikeTraceVariant::AssertEqReq)
            }
            _ if s == FnLikeTraceVariant::AssertNeReq.as_str() => {
                Ok(FnLikeTraceVariant::AssertNeReq)
            }
            _ if s == FnLikeTraceVariant::DebugAssertReq.as_str() => {
                Ok(FnLikeTraceVariant::DebugAssertReq)
            }
            _ if s == FnLikeTraceVariant::DebugAssertEqReq.as_str() => {
                Ok(FnLikeTraceVariant::DebugAssertEqReq)
            }
            _ if s == FnLikeTraceVariant::DebugAssertNeReq.as_str() => {
                Ok(FnLikeTraceVariant::DebugAssertNeReq)
            }
            _ if s == FnLikeTraceVariant::ClarifyReq.as_str() => Ok(FnLikeTraceVariant::ClarifyReq),
            _ if s == FnLikeTraceVariant::LinkReq.as_str() => Ok(FnLikeTraceVariant::LinkReq),
            _ => Err(anyhow::anyhow!(
                "Invalid function-like trace variant: {}",
                s
            )),
        }
    }
}

impl From<FnLikeTraceVariant> for TraceKind {
    fn from(value: FnLikeTraceVariant) -> Self {
        match value {
            FnLikeTraceVariant::SatisfyReq | FnLikeTraceVariant::ImplReq => TraceKind::Satisfies,
            FnLikeTraceVariant::VerifyReq
            | FnLikeTraceVariant::AssertReq
            | FnLikeTraceVariant::AssertEqReq
            | FnLikeTraceVariant::AssertNeReq
            | FnLikeTraceVariant::DebugAssertReq
            | FnLikeTraceVariant::DebugAssertEqReq
            | FnLikeTraceVariant::DebugAssertNeReq => TraceKind::Verifies,
            FnLikeTraceVariant::ClarifyReq => TraceKind::Clarifies,
            FnLikeTraceVariant::LinkReq => TraceKind::Links,
        }
    }
}
