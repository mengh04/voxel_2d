use bevy::prelude::*;

// --- 1. 配置常量 ---
const VOXEL_SIZE: f32 = 8.0; // 每个格子的大小
const GRID_WIDTH: usize = 100; // 地图宽（格子数）
const GRID_HEIGHT: usize = 80; // 地图高（格子数）
const ISO_LEVEL: f32 = 0.5; // 阈值：密度 > 0.5 认为是墙，< 0.5 是空气

// --- 2. 资源定义：地图数据 ---
#[derive(Resource)]
struct VoxelMap {
    data: Vec<f32>, // 扁平化的一维数组
    width: usize,
    height: usize,
}

impl VoxelMap {
    fn new(width: usize, height: usize) -> Self {
        Self {
            data: vec![1.0; width * height], // 初始化全是 1.0 (实心)
            width,
            height,
        }
    }

    // 辅助：获取世界坐标对应的网格坐标
    fn world_to_grid(&self, world_pos: Vec2) -> (i32, i32) {
        // 地图居中显示，所以需要加上宽高的一半作为偏移
        let offset_x = (self.width as f32 * VOXEL_SIZE) / 2.0;
        let offset_y = (self.height as f32 * VOXEL_SIZE) / 2.0;

        let x = ((world_pos.x + offset_x) / VOXEL_SIZE).floor() as i32;
        let y = ((world_pos.y + offset_y) / VOXEL_SIZE).floor() as i32;
        (x, y)
    }

    // 安全获取密度（越界返回 0.0）
    fn get_density(&self, x: i32, y: i32) -> f32 {
        if x < 0 || x >= self.width as i32 || y < 0 || y >= self.height as i32 {
            return 0.0;
        }
        self.data[y as usize * self.width + x as usize]
    }

    // 修改密度
    fn modify_density(&mut self, x: i32, y: i32, amount: f32) {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            let idx = y as usize * self.width + x as usize;
            self.data[idx] = (self.data[idx] + amount).clamp(0.0, 1.0);
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(VoxelMap::new(GRID_WIDTH, GRID_HEIGHT)) // 初始化地图
        .add_systems(Startup, setup)
        .add_systems(Update, (handle_input, draw_marching_squares)) // 核心系统
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d::default());
}

// --- 3. 系统：处理挖掘/填补 ---
fn handle_input(
    buttons: Res<ButtonInput<MouseButton>>,
    q_window: Query<&Window>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    mut map: ResMut<VoxelMap>,
) {
    let Ok(window) = q_window.single() else {
        return;
    };
    let Ok((camera, camera_transform)) = q_camera.single() else {
        return;
    };

    // 如果按下了左键(挖) 或 右键(填)
    let is_digging = buttons.pressed(MouseButton::Left);
    let is_building = buttons.pressed(MouseButton::Right);

    if is_digging || is_building {
        if let Some(cursor_pos) = window.cursor_position() {
            if let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
                // 找到鼠标所在的格子
                let (gx, gy) = map.world_to_grid(world_pos);
                let radius = 4; // 影响半径

                // 遍历周围的格子进行修改
                for dy in -radius..=radius {
                    for dx in -radius..=radius {
                        let dist = ((dx * dx + dy * dy) as f32).sqrt();
                        if dist <= radius as f32 {
                            // 简单的挖掘力度计算
                            let amount = if is_digging { -0.1 } else { 0.1 };
                            map.modify_density(gx + dx, gy + dy, amount);
                        }
                    }
                }
            }
        }
    }
}

// --- 4. 核心系统：Marching Squares 可视化 ---
// 这里是“平滑”魔法发生的地方
fn draw_marching_squares(map: Res<VoxelMap>, mut gizmos: Gizmos) {
    let offset_x = -(map.width as f32 * VOXEL_SIZE) / 2.0;
    let offset_y = -(map.height as f32 * VOXEL_SIZE) / 2.0;

    // 遍历每一个格子（作为正方形的左下角）
    for y in 0..map.height as i32 - 1 {
        for x in 0..map.width as i32 - 1 {
            // 1. 获取正方形四个角的坐标
            let p0 = Vec2::new(x as f32 * VOXEL_SIZE, y as f32 * VOXEL_SIZE)
                + Vec2::new(offset_x, offset_y); // 左下
            let p1 = p0 + Vec2::new(VOXEL_SIZE, 0.0); // 右下
            let p2 = p0 + Vec2::new(VOXEL_SIZE, VOXEL_SIZE); // 右上
            let p3 = p0 + Vec2::new(0.0, VOXEL_SIZE); // 左上

            // 2. 获取四个角的密度
            let v0 = map.get_density(x, y);
            let v1 = map.get_density(x + 1, y);
            let v2 = map.get_density(x + 1, y + 1);
            let v3 = map.get_density(x, y + 1);

            // 调试显示：画出原始数据点（红色小点）
            // 如果你把这几行注释掉，就只剩下平滑的线了
            if v0 > 0.0 {
                gizmos.circle_2d(p0, 1.0 + v0 * 2.0, Color::srgba(1.0, 0.0, 0.0, 0.3));
            }

            // 3. 计算“状态码” (Case Index)
            // 二进制编码：如果角是墙(>0.5)设为1，否则为0
            let mut case_index = 0;
            if v0 >= ISO_LEVEL {
                case_index |= 1;
            } // 左下位
            if v1 >= ISO_LEVEL {
                case_index |= 2;
            } // 右下位
            if v2 >= ISO_LEVEL {
                case_index |= 4;
            } // 右上位
            if v3 >= ISO_LEVEL {
                case_index |= 8;
            } // 左上位

            // 如果全空(0)或全满(15)，不需要画线
            if case_index == 0 || case_index == 15 {
                continue;
            }

            // 4. 【平滑的关键】计算插值点
            // 我们不取边的中点，而是根据密度比例计算准确位置
            let a = interpolate(p0, p3, v0, v3); // 左边
            let b = interpolate(p3, p2, v3, v2); // 上边
            let c = interpolate(p1, p2, v1, v2); // 右边
            let d = interpolate(p0, p1, v0, v1); // 下边

            // 5. 根据状态码画线 (这是 Marching Squares 的标准查找表逻辑)
            let color = Color::srgb(0.0, 1.0, 0.0); // 绿色墙壁线

            match case_index {
                1 => gizmos.line_2d(a, d, color),
                2 => gizmos.line_2d(d, c, color),
                3 => gizmos.line_2d(a, c, color),
                4 => gizmos.line_2d(c, b, color),
                5 => {
                    gizmos.line_2d(a, d, color);
                    gizmos.line_2d(b, c, color);
                }
                6 => gizmos.line_2d(d, b, color),
                7 => gizmos.line_2d(a, b, color),
                8 => gizmos.line_2d(a, b, color),
                9 => gizmos.line_2d(d, b, color),
                10 => {
                    gizmos.line_2d(a, b, color);
                    gizmos.line_2d(c, d, color);
                }
                11 => gizmos.line_2d(c, b, color),
                12 => gizmos.line_2d(a, c, color),
                13 => gizmos.line_2d(d, c, color),
                14 => gizmos.line_2d(a, d, color),
                _ => {}
            }
        }
    }
}

// 【魔法函数】线性插值
// 计算 "0.5" 到底在 p1 和 p2 连线的什么位置
fn interpolate(p1: Vec2, p2: Vec2, v1: f32, v2: f32) -> Vec2 {
    if (v2 - v1).abs() < 0.0001 {
        return p1;
    } // 防止除以0
    let t = (ISO_LEVEL - v1) / (v2 - v1);
    p1 + (p2 - p1) * t
}
