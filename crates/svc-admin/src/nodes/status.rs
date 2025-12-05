use crate::dto::node::AdminStatusView;

// TODO: normalize node health/ready/version/status into AdminStatusView.
pub fn build_status_placeholder() -> AdminStatusView {
    AdminStatusView::placeholder()
}
