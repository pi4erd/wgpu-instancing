use bytemuck::{Pod, Zeroable};
use cgmath::Zero;
use wgpu::util::DeviceExt;

pub trait Drawable {
    fn draw(&self, render_pass: &mut wgpu::RenderPass);
}

pub struct Model<T> {
    data: T,
    pub position: cgmath::Vector3<f32>,
    pub rotation: cgmath::Quaternion<f32>,
    pub scale: cgmath::Vector3<f32>,
    bind_group: wgpu::BindGroup,
    model_buffer: wgpu::Buffer,
}

impl<T> Model<T> {
    pub fn new(bg_layout: &wgpu::BindGroupLayout, device: &wgpu::Device, data: T) -> Self {
        // TODO: Stop creating buffers for each model
        let model_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[ModelUniform::default()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: bg_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: model_buffer.as_entire_binding(),
            }],
        });

        Self {
            data,
            position: cgmath::Vector3::new(0.0, 0.0, 0.0),
            rotation: cgmath::Quaternion::zero(),
            scale: cgmath::Vector3::new(1.0, 1.0, 1.0),
            model_buffer,
            bind_group,
        }
    }

    pub fn update_buffer(&self, queue: &wgpu::Queue) {
        queue.write_buffer(
            &self.model_buffer,
            0,
            bytemuck::cast_slice(&[self.uniform()]),
        );
    }

    pub fn uniform(&self) -> ModelUniform {
        let model = cgmath::Matrix4::from_translation(self.position)
            * cgmath::Matrix4::from_axis_angle(self.rotation.v, cgmath::Rad(self.rotation.s))
            * cgmath::Matrix4::from_nonuniform_scale(self.scale.x, self.scale.y, self.scale.z);

        ModelUniform {
            model: model.into(),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct ModelUniform {
    model: [[f32; 4]; 4],
}

impl<T: Drawable> Drawable for Model<T> {
    fn draw(&self, render_pass: &mut wgpu::RenderPass) {
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        self.data.draw(render_pass);
    }
}
