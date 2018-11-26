use ::router;

#[derive(Clone, Copy)]
pub(in auth_handler) enum Route {
    Login,
    Logout,
    Info,
    Check
}

pub(in auth_handler) fn create_routing_table() -> router::RoutingTable<Route> {
    let mut r = router::RoutingTable::new();
    r.insert("/info", Route::Info);
    r.insert("/login", Route::Login);
    r.insert("/logout", Route::Logout);
    r.insert("/check", Route::Check);
    r
}