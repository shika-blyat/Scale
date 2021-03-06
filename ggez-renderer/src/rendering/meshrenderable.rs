use crate::rendering::render_context::RenderContext;
use ggez::graphics::Color;
use scale::physics::Transform;
use scale::rendering::meshrender_component::{
    CircleRender, LineRender, LineToRender, MeshRenderEnum, RectRender,
};
use scale::specs::ReadStorage;

pub trait MeshRenderable: Send + Sync {
    fn draw(&self, trans: &Transform, transforms: &ReadStorage<Transform>, rc: &mut RenderContext);
}

impl MeshRenderable for MeshRenderEnum {
    fn draw(&self, trans: &Transform, transforms: &ReadStorage<Transform>, rc: &mut RenderContext) {
        match self {
            MeshRenderEnum::Circle(x) => x.draw(trans, transforms, rc),
            MeshRenderEnum::Rect(x) => x.draw(trans, transforms, rc),
            MeshRenderEnum::LineTo(x) => x.draw(trans, transforms, rc),
            MeshRenderEnum::Line(x) => x.draw(trans, transforms, rc),
        }
    }
}

impl MeshRenderable for CircleRender {
    fn draw(&self, pos: &Transform, _: &ReadStorage<Transform>, rc: &mut RenderContext) {
        rc.tess.color = scale_color(self.color);
        rc.tess.set_filled(self.filled);
        rc.tess.draw_circle(pos.project(self.offset), self.radius);
    }
}

impl MeshRenderable for RectRender {
    fn draw(&self, trans: &Transform, _: &ReadStorage<Transform>, rc: &mut RenderContext) {
        rc.tess.color = scale_color(self.color);
        rc.tess.set_filled(self.filled);
        let rect_pos = trans.position() + trans.apply_rotation(self.offset);
        rc.tess
            .draw_rect_cos_sin(rect_pos, self.width, self.height, trans.direction());
    }
}

impl MeshRenderable for LineToRender {
    fn draw(&self, trans: &Transform, transforms: &ReadStorage<Transform>, rc: &mut RenderContext) {
        let e = self.to;
        let pos2 = transforms.get(e).unwrap().position();
        rc.tess.color = scale_color(self.color);
        rc.tess.draw_stroke(trans.position(), pos2, self.thickness);
    }
}

impl MeshRenderable for LineRender {
    fn draw(&self, trans: &Transform, _: &ReadStorage<Transform>, rc: &mut RenderContext) {
        let start = trans.position();
        let end = start + self.offset;
        rc.tess.color = scale_color(self.color);
        rc.tess.draw_stroke(start, end, self.thickness);
    }
}

pub fn scale_color(color: scale::rendering::Color) -> Color {
    Color {
        r: color.r,
        g: color.g,
        b: color.b,
        a: color.a,
    }
}
