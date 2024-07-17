use maud::{html, Markup, PreEscaped, DOCTYPE};

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
                main #"main_content" ."my-2"."lg:mx-4" { @if let Some(content) = main_content { (content) } }
                dialog #"modal"."modal"."modal-bottom"."sm:modal-middle" {
                    ."modal-box" {
                        form method="dialog" { button ."btn"."btn-sm"."btn-circle"."btn-ghost"."absolute"."right-2"."top-2" {"✕"} }
                        progress #"modal-loading"."progress"."mt-6"."[&:has(+#modal-content:not(:empty)):not(.htmx-request)]:hidden" {}
                        div #"modal-content" {}
                    }
                    script {(PreEscaped("function openModal() { $('#modal-content').empty(); $('#modal')[0].showModal(); }"))}
                    form method="dialog" ."modal-backdrop" { button {"CLOSE"} }
                }
                #"alerts"."toast" {}
                script src="https://unpkg.com/htmx.org@1.9.9" {}
                script src="https://code.jquery.com/jquery-3.7.1.slim.min.js" {}
            }
        }
    }
}
