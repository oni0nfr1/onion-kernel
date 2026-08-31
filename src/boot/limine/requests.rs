use limine::{
    BaseRevision, RequestsEndMarker, RequestsStartMarker,
    paging::PagingMode,
    request::{
        ExecutableAddressRequest, FramebufferRequest, HhdmRequest, MemmapRequest, PagingModeRequest,
    },
};

#[used]
#[unsafe(link_section = ".limine_requests_start")]
pub static REQUESTS_START: RequestsStartMarker = RequestsStartMarker::new();

#[used]
#[unsafe(link_section = ".limine_requests")]
pub static BASE_REVISION: BaseRevision = BaseRevision::new();

#[used]
#[unsafe(link_section = ".limine_requests")]
pub static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

#[used]
#[unsafe(link_section = ".limine_requests")]
pub static MEMMAP_REQUEST: MemmapRequest = MemmapRequest::new();

#[used]
#[unsafe(link_section = ".limine_requests")]
pub static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();

#[used]
#[unsafe(link_section = ".limine_requests")]
pub static EXECUTABLE_ADDRESS_REQUEST: ExecutableAddressRequest = ExecutableAddressRequest::new();

#[used]
#[unsafe(link_section = ".limine_requests")]
pub static PAGING_MODE_REQUEST: PagingModeRequest =
    PagingModeRequest::new_exact(PagingMode::X86_64_4LVL);

#[used]
#[unsafe(link_section = ".limine_requests_end")]
pub static REQUESTS_END: RequestsEndMarker = RequestsEndMarker::new();
