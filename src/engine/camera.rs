// 'camera.rs'

use std::ops::Sub;
use glam::{Mat4, Quat, Vec2, Vec3};

pub struct CameraPerspective {
    pub width: f32,
    pub height: f32,
    pub near: f32,
    pub far: f32,
    pub fov: f32,
    
    pub position: Vec3,

    pub projection: Mat4,
    pub view: Mat4,
    pub combined: Mat4,

    pub direction: Vec3,
    pub orientation: Quat,
}

impl CameraPerspective {
    pub fn new() -> Self {
        Self {
            width: 0.0,
            height: 0.0,
            near: 0.1,
            far: 100.0,
            fov: 90.0,
            
            position: Vec3::ZERO,

            projection: Mat4::IDENTITY,
            view: Mat4::IDENTITY,
            combined: Mat4::IDENTITY,

            direction: Vec3::ZERO,
            orientation: Quat::IDENTITY,
        }
    }

    pub fn update(&mut self) {
        let forward = self.orientation * Vec3::Z;
        let up = self.orientation * Vec3::Y;
        self.view = Mat4::look_to_lh(self.position, forward, up);

        self.update_combined();
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
        self.update_projection();
    }

    fn update_projection(&mut self) {
        let aspect = self.width / self.height;

        self.projection = Mat4::perspective_lh(
            self.fov.to_radians(),
            aspect,
            self.near,
            self.far,
        );
        self.update_combined();
    }

    fn update_combined(&mut self) {
        self.combined = self.projection * self.view;
    }

}

pub struct CameraOrthographic {
    pub width: f32,
    pub height: f32,
    pub near: f32,
    pub far: f32,

    origin: Vec2,

    pub position: Vec3,
    pub scale: Vec3,

    pub projection: Mat4,
    pub view: Mat4,
    pub combined: Mat4,
}

impl CameraOrthographic {
    pub fn new() -> Self {
        Self {
            width: 0.0,
            height: 0.0,
            near: -1.0,
            far: 1.0,

            origin: Vec2::ZERO,

            position: Vec3::ZERO,
            scale: Vec3::ONE,

            projection: Mat4::IDENTITY,
            view: Mat4::IDENTITY,
            combined: Mat4::IDENTITY,
        }
    }

    pub fn set_origin(&mut self, origin: Vec2) {
        self.origin = origin;
        self.update_projection();
    }

    pub fn origin(&self) -> Vec2 {
        self.origin
    }
    
    
    pub fn update(&mut self) {
        let world = Mat4::from_scale_rotation_translation(self.scale, Quat::IDENTITY, self.position);
        self.view = world.inverse();
        self.update_combined();
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
        self.update_projection();
    }
    
    fn update_projection(&mut self) {
        let inv_origin = Vec2::ONE.sub(self.origin);
        
        let left = -self.width * self.origin.x;
        let right = self.width * inv_origin.x;
        let bottom = self.height * inv_origin.y;
        let top = -self.height * self.origin.y;
        
        self.projection = Mat4::orthographic_lh(
            left,
            right,
            bottom,
            top,
            self.near,
            self.far,
        );
        self.update_combined();
    }

    fn update_combined(&mut self) {
        self.combined = self.projection * self.view;
    }
}
