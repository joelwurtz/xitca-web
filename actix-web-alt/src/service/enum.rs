use std::{
    future::Future,
    marker::PhantomData,
    task::{Context, Poll},
};

use actix_service_alt::Service;

use crate::request::WebRequest;

macro_rules! enum_service ({ $($param:ident),+ } => {
    pub enum EnumService<State, $($param),+> {
        #[doc(hidden)]
        _State(PhantomData<State>),
        $($param($param)), +
    }

    impl<'r, State, Res, Err, $($param),+> Service<WebRequest<'r, State>> for EnumService<State, $($param),+>
    where
        State: 'static,
        Res: 'static,
        Err: 'static,
        $($param: Service<WebRequest<'r, State>, Response = Res, Error = Err> + 'static), +
    {
        type Response = Res;
        type Error = Err;
        type Future<'f> = impl Future<Output = Result<Self::Response, Self::Error>>;

        fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            match *self {
                Self::_State(_) => unreachable!(""),
                $(Self::$param(ref s) => s.poll_ready(cx)), +
            }
        }

        fn call<'c>(&'c self, req: WebRequest<'r, State>) -> Self::Future<'c>
        where
            'r: 'c
        {
            async move {
                match *self {
                    Self::_State(_) => unreachable!(""),
                    $(Self::$param(ref s) => s.call(req).await), +
                }
            }
        }
    }
});

enum_service! { A, B, C, D, E, F, G, H, I, J, K, L }