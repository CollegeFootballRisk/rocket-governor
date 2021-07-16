#![deny(unsafe_code)]
#![deny(warnings)]
#![deny(clippy::all)]

use rocket::{
    catchers, get,
    http::{Header, Status},
    launch,
    local::blocking::Client,
    routes,
};
use rocket_governor::{Method, Quota, RocketGovernable};
use rocket_governor_derive::{RocketGovernor, RocketGovernorWithMember};
use std::{thread, time::Duration};

#[derive(RocketGovernor)]
pub struct RateLimitGuard;

impl<'r> RocketGovernable<'r> for RateLimitGuard {
    fn quota(_method: Method, _route_name: &str) -> Quota {
        Quota::per_second(Self::nonzero(1u32))
    }
}

#[derive(RocketGovernorWithMember)]
pub struct RateLimitGuardWithMember {
    pub member: u8,
}

impl<'r> RocketGovernable<'r> for RateLimitGuardWithMember {
    fn quota(_method: Method, _route_name: &str) -> Quota {
        Quota::with_period(Duration::from_secs(2u64)).unwrap()
    }
}

impl Default for RateLimitGuardWithMember {
    fn default() -> Self {
        Self { member: 254u8 }
    }
}

#[get("/")]
fn route_test(_limitguard: RateLimitGuard) -> Status {
    Status::Ok
}

#[get("/member")]
fn route_member(limitguard: RateLimitGuardWithMember) -> Status {
    let _ = limitguard.member;
    Status::Ok
}

#[launch]
fn launch_rocket() -> _ {
    rocket::build()
        .mount("/", routes![route_test, route_member])
        .register("/", catchers!(ratelimitguard_rocket_governor_catcher))
}

#[test]
fn test_ratelimit() {
    let client = Client::untracked(launch_rocket()).expect("no rocket instance");
    let mut req = client.get("/");
    req.add_header(Header::new("X-Real-IP", "127.0.1.1"));
    let res = req.dispatch();

    assert_eq!(Status::Ok, res.status());

    let mut req = client.get("/");
    req.add_header(Header::new("X-Real-IP", "127.0.1.1"));
    let res = req.dispatch();

    assert_eq!(Status::TooManyRequests, res.status());

    let mut req = client.get("/");
    req.add_header(Header::new("X-Real-IP", "127.0.1.1"));
    let res = req.dispatch();

    assert_eq!(Status::TooManyRequests, res.status());

    let mut req = client.get("/");
    req.add_header(Header::new("X-Real-IP", "127.0.1.2"));
    let res = req.dispatch();

    assert_eq!(Status::Ok, res.status());

    thread::sleep(Duration::from_millis(1100u64));

    let mut req = client.get("/");
    req.add_header(Header::new("X-Real-IP", "127.0.1.1"));
    let res = req.dispatch();

    assert_eq!(Status::Ok, res.status());

    thread::sleep(Duration::from_millis(300u64));

    let mut req = client.get("/");
    req.add_header(Header::new("X-Real-IP", "127.0.1.1"));
    let res = req.dispatch();

    assert_eq!(Status::TooManyRequests, res.status());
}

#[test]
fn test_ratelimit_with_member() {
    let client = Client::untracked(launch_rocket()).expect("no rocket instance");
    let mut req = client.get("/member");
    req.add_header(Header::new("X-Real-IP", "127.0.2.1"));
    let res = req.dispatch();

    assert_eq!(Status::Ok, res.status());

    let mut req = client.get("/member");
    req.add_header(Header::new("X-Real-IP", "127.0.2.2"));
    let res = req.dispatch();

    assert_eq!(Status::Ok, res.status());

    let mut req = client.get("/member");
    req.add_header(Header::new("X-Real-IP", "127.0.2.1"));
    let res = req.dispatch();

    assert_eq!(Status::TooManyRequests, res.status());

    thread::sleep(Duration::from_millis(1100u64));

    let mut req = client.get("/member");
    req.add_header(Header::new("X-Real-IP", "127.0.2.1"));
    let res = req.dispatch();

    assert_eq!(Status::TooManyRequests, res.status());

    thread::sleep(Duration::from_millis(900u64));

    let mut req = client.get("/member");
    req.add_header(Header::new("X-Real-IP", "127.0.2.1"));
    let res = req.dispatch();

    assert_eq!(Status::Ok, res.status());
}