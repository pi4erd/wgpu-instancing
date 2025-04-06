use std::ops::Range;

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

pub trait Vertex: Pod + Zeroable {
    fn attribs() -> &'static [wgpu::VertexAttribute];
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::attribs(),
        }
    }
}

pub trait Instance: Pod + Zeroable {
    fn attribs() -> &'static [wgpu::VertexAttribute];
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: Self::attribs(),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct DefaultVertex3d {
    pub position: [f32; 3],
}

impl DefaultVertex3d {
    const ATTRIBS: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
        0 => Float32x3,
    ];
}

impl Vertex for DefaultVertex3d {
    fn attribs() -> &'static [wgpu::VertexAttribute] {
        Self::ATTRIBS
    }
}

pub struct Mesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    element_count: usize,
}

#[allow(dead_code)]
impl Mesh {
    pub fn create(device: &wgpu::Device, vertices: &[impl Vertex], indices: &[u32]) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        log::debug!(
            "Created model with {} vertices and {} triangles.",
            vertices.len(),
            indices.len()
        );

        Self {
            vertex_buffer,
            index_buffer,
            element_count: indices.len(),
        }
    }

    pub fn draw_instanced(
        &self,
        render_pass: &mut wgpu::RenderPass,
        instance_buffer: &wgpu::Buffer,
        instances: Range<u32>,
    ) {
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        render_pass.draw_indexed(0..self.element_count as u32, 0, instances);
    }

    pub fn draw(&self, render_pass: &mut wgpu::RenderPass) {
        // TODO: Move to bundle?
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        render_pass.draw_indexed(0..self.element_count as u32, 0, 0..1);
    }
}
