use axum::response::IntoResponse;
use maud::{html, Markup, PreEscaped, Render, DOCTYPE};

use crate::icons;

pub fn layout(navbar_options: Markup, main_content: Option<Markup>) -> Markup {
    html! {
        (DOCTYPE)
        html {
            head {
                meta charset="UTF-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                meta http-equiv="X-UA-Compatible" content="ie=edge";
                title {"Membership Database Management Application"}
                link rel="stylesheet" href="/assets/styles.css";
            }
            body {
                header ."navbar"."bg-base-300"."lg:rounded-box"."lg:m-3"."lg:w-auto" {
                    (navbar_options)
                }
                main #"main_content" ."my-2"."lg:mx-4" hx-on--before-swap="event.target == this && $('#action_buttons').empty()" { @if let Some(content) = main_content { (content) } }
                dialog #"modal"."modal"."modal-bottom"."sm:modal-middle" {
                    ."modal-box" {
                        form method="dialog" { button ."btn"."btn-sm"."btn-circle"."btn-ghost"."absolute"."right-2"."top-2" {"✕"} }
                        progress #"modal-loading"."progress"."mt-6"."[&:has(+#modal-content:not(:empty)):not(.htmx-request)]:hidden" {}
                        div #"modal-content" {}
                    }
                    script {(PreEscaped("function openModal() { $('#modal-content').empty(); $('#modal')[0].showModal(); }"))}
                    form method="dialog" ."modal-backdrop" { button {"CLOSE"} }
                }
                #"alerts"."toast"."*:w-fit"."items-end" {
                    #"action_buttons" ."*:ml-2" {}
                }
                script src="https://unpkg.com/htmx.org@2.0.1" {}
                script src="https://code.jquery.com/jquery-3.7.1.slim.min.js" {}
            }
        }
    }
}

pub enum ToastAlert<'a> {
    Success(&'a str),
    Error(&'a str),
}

impl Render for ToastAlert<'_> {
    fn render(&self) -> Markup {
        let toastid = sqlx::types::Uuid::new_v4().simple();
        let classname = match self {
            Self::Success(_) => "alert-success",
            Self::Error(_) => "alert-error",
        };

        html! {div hx-swap-oob="afterbegin:#alerts" {
            #{"toast_"(toastid)}."alert"."transition-opacity"."duration-300".(classname) role="alert" {
                @match self {
                    Self::Success(text) => (icons::success()) span {(text)},
                    Self::Error(text) => (icons::error()) span {(text)},
                }
                script {(PreEscaped(format!("
                    setTimeout(() => {{
                        const toastElem = $('#toast_{}');
                        toastElem.on('transitionend', (event) => {{event.target.remove();}});
                        toastElem.css('opacity', 0);
                    }}, 2500);
                ", toastid)))}
            }
        }}
    }
}

impl IntoResponse for ToastAlert<'_> {
    fn into_response(self) -> axum::response::Response {
        self.render().into_response()
    }
}
