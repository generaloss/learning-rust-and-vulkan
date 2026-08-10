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

    pub orientation: Quat,
}

impl CameraPerspective {
    pub fn new() -> Self {
        Self {
            width: 0.0,
            height: 0.0,
            near: 0.1,
            far: 1000.0,
            fov: 90.0,
            
            position: Vec3::ZERO,

            projection: Mat4::IDENTITY,
            view: Mat4::IDENTITY,
            combined: Mat4::IDENTITY,

            orientation: Quat::IDENTITY,
        }
    }

    pub fn update(&mut self) {
        self.update_view();
        self.update_combined();
    }

    fn update_view(&mut self) {
        self.view = Mat4::from_rotation_translation(self.orientation, self.position).inverse();
    }

    fn update_combined(&mut self) {
        self.combined = self.projection * self.view;
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
        self.update_projection();
    }

    fn update_projection(&mut self) {
        let aspect = self.width / self.height;

        self.projection = Mat4::perspective_rh(
            self.fov.to_radians(),
            aspect,
            self.near,
            self.far,
        );

        self.projection.y_axis.y *= -1.0;

        self.update_combined();
    }

    fn orientation(&self) -> Quat {
        self.orientation
    }

    pub fn set_euler_angles(&mut self, yaw_rad: f32, pitch_rad: f32) {
        let yaw_quat = Quat::from_rotation_y(yaw_rad);
        let pitch_quat = Quat::from_rotation_x(pitch_rad);

        self.orientation = (yaw_quat * pitch_quat).normalize();
    }

    #[inline]
    pub fn forward(&self) -> Vec3 {
        self.orientation * Vec3::NEG_Z
    }

    #[inline]
    pub fn backward(&self) -> Vec3 {
        -self.forward()
    }

    #[inline]
    pub fn right(&self) -> Vec3 {
        self.orientation * Vec3::X
    }

    #[inline]
    pub fn left(&self) -> Vec3 {
        -self.right()
    }

    #[inline]
    pub fn up(&self) -> Vec3 {
        self.orientation * Vec3::Y
    }

    #[inline]
    pub fn down(&self) -> Vec3 {
        -self.up()
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
