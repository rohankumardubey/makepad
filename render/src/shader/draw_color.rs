use {
    makepad_shader_compiler::makepad_live_compiler::*,
    makepad_derive_live::*,
    crate::{
        cx::Cx,
        live_traits::*,
        shader::draw_quad::DrawQuad
    },
};

live_register!{
    DrawColor: {{DrawColor}} {
        fn pixel(self) -> vec4 {
            return vec4(self.color.rgb*self.color.a, self.color.a);
        }
    }
}

#[derive(Live, LiveHook)]
#[repr(C)]
pub struct DrawColor {
    #[live()] pub deref_target: DrawQuad,
    #[live()] pub color: Vec4
}