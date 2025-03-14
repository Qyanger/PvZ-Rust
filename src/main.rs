use ggez::{
    Context, ContextBuilder, GameResult, timer,
    graphics::{self, Color, DrawParam, Rect, Canvas},
    event::{self, EventHandler},
    input::mouse,
    mint::Point2,
    glam::Vec2,
};
use std::collections::VecDeque;
use rand::Rng;

// 游戏配置
const WINDOW_WIDTH: f32 = 800.0;
const WINDOW_HEIGHT: f32 = 600.0;
const TOOLBAR_HEIGHT: f32 = 100.0; // 顶部工具栏高度
const GRID_ROWS: usize = 5;        // 5行战斗区域
const GRID_COLUMNS: usize = 9;      // 9列格子
const CELL_SIZE: f32 = 80.0;        // 格子大小
const SUN_PRODUCE_INTERVAL: u32 = 120; // 向日葵产阳光间隔
const SUN_FALL_SPEED: f32 = 0.5;    // 阳光下落速度
const PEASHOOTER_SHOOT_INTERVAL: u32 = 60; // 豌豆射手发射间隔
const PEASHOOTER_BULLET_SPEED: f32 = 2.0; // 豌豆子弹速度
const DEFAULT_ZOMBIE_SPEED: f32 = 1.0; // 默认僵尸速度

// 植物类型
#[derive(Clone, Copy, Debug, PartialEq)]
enum PlantType {
    Sunflower, // 向日葵（产阳光）
    Peashooter, // 豌豆射手（攻击）
}

// 游戏状态
struct MyGame {
    selected_plant: Option<PlantType>, // 选中的植物
    sun: u32,                         // 阳光数量
    plants: Vec<Plant>,               // 已放置的植物
    zombies: VecDeque<Zombie>,        // 僵尸队列
    spawn_timer: u32,                 // 僵尸生成计时器
    sun_timer: u32,                   // 阳光生产计时器
    suns: Vec<Sun>,                   // 生成的阳光
    bullets: Vec<Bullet>,             // 豌豆子弹
    game_over: bool,                  // 游戏是否结束
}

// 植物结构体（带类型）
struct Plant {
    cell: (usize, usize), // (行, 列)
    plant_type: PlantType,
    health: u32,
    last_sun_time: u32,   // 向日葵上次产阳光时间
    last_shoot_time: u32, // 豌豆射手上次发射时间
}

// 僵尸结构体
struct Zombie {
    position: Vec2,       // 实际屏幕坐标
    speed: f32,           // 僵尸速度，可修改
    health: u32,
    is_blocked: bool,     // 标记僵尸是否被阻挡
}

// 阳光结构体
struct Sun {
    position: Vec2,
    is_collected: bool,
    fall_timer: u32,      // 阳光下落计时器
}

// 子弹结构体
struct Bullet {
    position: Vec2,
    row: usize,           // 子弹所在行
}

impl MyGame {
    pub fn new(ctx: &mut Context) -> MyGame {
        // 初始化工具栏植物（向日葵和豌豆射手）
        MyGame {
            selected_plant: None,
            sun: 50,
            plants: Vec::new(),
            zombies: VecDeque::new(),
            spawn_timer: 0,
            sun_timer: 0,
            suns: Vec::new(),
            bullets: Vec::new(),
            game_over: false,
        }
    }

    // 坐标转格子
    fn screen_to_cell(x: f32, y: f32) -> Option<(usize, usize)> {
        // 忽略工具栏区域
        if y < TOOLBAR_HEIGHT {
            return None;
        }

        let row = ((y - TOOLBAR_HEIGHT) / CELL_SIZE) as usize;
        let col = (x / CELL_SIZE) as usize;

        if row < GRID_ROWS && col < GRID_COLUMNS {
            Some((row, col))
        } else {
            None
        }
    }

    // 格子转屏幕坐标
    fn cell_to_screen((row, col): (usize, usize)) -> Vec2 {
        Vec2::new(
            col as f32 * CELL_SIZE + CELL_SIZE / 2.0,
            row as f32 * CELL_SIZE + TOOLBAR_HEIGHT + CELL_SIZE / 2.0,
        )
    }

    // 获取植物价格
    fn get_plant_cost(plant_type: PlantType) -> u32 {
        match plant_type {
            PlantType::Sunflower => 50,
            PlantType::Peashooter => 100,
        }
    }
}

impl EventHandler for MyGame {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        if self.game_over {
            return Ok(());
        }

        // 阳光生产（每2秒一次）
        self.sun_timer += 1;
        for plant in self.plants.iter_mut() {
            if plant.plant_type == PlantType::Sunflower {
                plant.last_sun_time += 1;
                if plant.last_sun_time >= SUN_PRODUCE_INTERVAL {
                    plant.last_sun_time = 0;
                    let plant_pos = MyGame::cell_to_screen(plant.cell);
                    self.suns.push(Sun {
                        position: plant_pos,
                        is_collected: false,
                        fall_timer: 0,
                    });
                }
            }
        }

        // 阳光下落动画
        for sun in self.suns.iter_mut() {
            if!sun.is_collected {
                sun.fall_timer += 1;
                sun.position.y += SUN_FALL_SPEED;
            }
        }

        // 豌豆射手发射子弹
        for plant in self.plants.iter_mut() {
            if plant.plant_type == PlantType::Peashooter {
                plant.last_shoot_time += 1;
                if plant.last_shoot_time >= PEASHOOTER_SHOOT_INTERVAL {
                    plant.last_shoot_time = 0;
                    let plant_pos = MyGame::cell_to_screen(plant.cell);
                    self.bullets.push(Bullet {
                        position: plant_pos,
                        row: plant.cell.0,
                    });
                }
            }
        }

        // 子弹移动
        for bullet in self.bullets.iter_mut() {
            bullet.position.x += PEASHOOTER_BULLET_SPEED;
        }

        // 子弹与僵尸碰撞检测
        let mut bullets_to_remove = Vec::new();
        let mut zombies_to_remove = Vec::new();
        for (i, bullet) in self.bullets.iter_mut().enumerate() {
            for (j, zombie) in self.zombies.iter_mut().enumerate() {
                if bullet.row == ((zombie.position.y - TOOLBAR_HEIGHT) / CELL_SIZE) as usize
                    && (zombie.position.x - bullet.position.x).abs() < 20.0
                    && (zombie.position.y - bullet.position.y).abs() < 20.0
                {
                    zombie.health = zombie.health.saturating_sub(10);
                    if zombie.health == 0 {
                        zombies_to_remove.push(j);
                    }
                    bullets_to_remove.push(i);
                    break;
                }
            }
        }
        // 移除被击中的僵尸和子弹
        for &i in bullets_to_remove.iter().rev() {
            self.bullets.remove(i);
        }
        for &j in zombies_to_remove.iter().rev() {
            self.zombies.remove(j);
        }

        // 僵尸移动
        for zombie in self.zombies.iter_mut() {
            if!zombie.is_blocked {
                zombie.position.x -= zombie.speed;
            }
        }

        // 碰撞检测（僵尸攻击植物）
        let mut plants_to_remove = Vec::new();
        for (i, plant) in self.plants.iter_mut().enumerate() {
            let plant_pos = MyGame::cell_to_screen(plant.cell);
            for zombie in self.zombies.iter_mut() {
                if (zombie.position.x - plant_pos.x).abs() < 25.0 && (zombie.position.y - plant_pos.y).abs() < 35.0 {
                    zombie.is_blocked = true;
                    plant.health = plant.health.saturating_sub(1);
                    if plant.health == 0 {
                        plants_to_remove.push(i);
                    }
                } else {
                    zombie.is_blocked = false;
                }
            }
        }
        for i in plants_to_remove {
            for zombie in self.zombies.iter_mut() {
                let plant_pos = MyGame::cell_to_screen(self.plants[i].cell);
                if (zombie.position.x - plant_pos.x).abs() < 25.0 && (zombie.position.y - plant_pos.y).abs() < 35.0 {
                    zombie.is_blocked = false;
                }
            }
            self.plants.remove(i);
        }

        // 检查僵尸是否走出界面
        for zombie in &self.zombies {
            if zombie.position.x < 0.0 {
                self.game_over = true;
                break;
            }
        }

        // 移除超出屏幕的僵尸
        self.zombies.retain(|z| z.position.x > 0.0);

        // 生成僵尸（每3秒一行）
        self.spawn_timer += 1;
        if self.spawn_timer >= 360 {
            self.spawn_timer = 0;
            let row = rand::thread_rng().gen_range(0..GRID_ROWS);
            let y = row as f32 * CELL_SIZE + TOOLBAR_HEIGHT + CELL_SIZE / 2.0;
            let speed = DEFAULT_ZOMBIE_SPEED * (rand::thread_rng().gen_range(0.8..1.2)); // 速度有一定随机波动
            self.zombies.push_back(Zombie {
                position: Vec2::new(WINDOW_WIDTH, y),
                speed,
                health: 50,
                is_blocked: false,
            });
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let mut canvas = graphics::Canvas::from_frame(ctx, Color::WHITE);

        if self.game_over {
            let game_over_text = graphics::Text::new("Game Over!");
            canvas.draw(
                &game_over_text,
                DrawParam::default()
                   .dest(Vec2::new(WINDOW_WIDTH / 2.0 - 50.0, WINDOW_HEIGHT / 2.0))
                   .color(Color::RED),
            );
            return canvas.finish(ctx);
        }

        // 绘制工具栏
        let toolbar_mesh = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::fill(),
            Rect::new(0.0, 0.0, WINDOW_WIDTH, TOOLBAR_HEIGHT),
            Color::from_rgb(211, 211, 211),
        )?;
        canvas.draw(&toolbar_mesh, DrawParam::default());

        // 绘制植物选择按钮
        self.draw_plant_selector(&mut canvas, ctx, PlantType::Sunflower, 100.0, 20.0);
        self.draw_plant_selector(&mut canvas, ctx, PlantType::Peashooter, 200.0, 20.0);

        // 绘制取消选择按钮
        self.draw_cancel_button(&mut canvas, ctx, 300.0, 20.0);

        // 绘制阳光显示
        let sun_text = graphics::Text::new(format!("Sun: {}", self.sun));
        canvas.draw(
            &sun_text,
            DrawParam::default()
               .dest(Vec2::new(WINDOW_WIDTH - 100.0, 20.0))
               .color(Color::YELLOW),
        );

        // 绘制战斗网格
        for row in 0..GRID_ROWS {
            for col in 0..GRID_COLUMNS {
                let (x, y) = (col as f32 * CELL_SIZE, row as f32 * CELL_SIZE + TOOLBAR_HEIGHT);
                let grid_mesh = graphics::Mesh::new_rectangle(
                    ctx,
                    graphics::DrawMode::stroke(2.0),
                    Rect::new(x, y, CELL_SIZE, CELL_SIZE),
                    Color::from_rgb(169, 169, 169),
                )?;
                canvas.draw(&grid_mesh, DrawParam::default());
            }
        }

        // 绘制植物
        for plant in &self.plants {
            let plant_pos = MyGame::cell_to_screen(plant.cell);
            match plant.plant_type {
                PlantType::Sunflower => {
                    let sunflower_mesh = graphics::Mesh::new_circle(
                        ctx,
                        graphics::DrawMode::fill(),
                        plant_pos,
                        CELL_SIZE / 2.0 - 5.0,
                        32.0,
                        Color::YELLOW,
                    )?;
                    canvas.draw(&sunflower_mesh, DrawParam::default());
                }
                PlantType::Peashooter => {
                    let peashooter_mesh = graphics::Mesh::new_rectangle(
                        ctx,
                        graphics::DrawMode::fill(),
                        Rect::new(plant_pos.x - 20.0, plant_pos.y - 30.0, 40.0, 60.0),
                        Color::GREEN,
                    )?;
                    canvas.draw(&peashooter_mesh, DrawParam::default());
                }
            }
        }

        // 绘制僵尸
        for zombie in &self.zombies {
            let zombie_mesh = graphics::Mesh::new_rectangle(
                ctx,
                graphics::DrawMode::fill(),
                Rect::new(zombie.position.x - 25.0, zombie.position.y - 35.0, 50.0, 70.0),
                Color::from_rgb(128, 128, 128),
            )?;
            canvas.draw(&zombie_mesh, DrawParam::default());
        }

        // 绘制阳光
        for sun in &self.suns {
            if!sun.is_collected {
                let sun_mesh = graphics::Mesh::new_circle(
                    ctx,
                    graphics::DrawMode::fill(),
                    sun.position,
                    20.0,
                    32.0,
                    Color::YELLOW,
                )?;
                canvas.draw(&sun_mesh, DrawParam::default());
            }
        }

        // 绘制子弹
        for bullet in &self.bullets {
            let bullet_mesh = graphics::Mesh::new_circle(
                ctx,
                graphics::DrawMode::fill(),
                bullet.position,
                10.0,
                32.0,
                Color::GREEN,
            )?;
            canvas.draw(&bullet_mesh, DrawParam::default());
        }

        // 绘制选中植物预览
        if let Some(plant_type) = self.selected_plant {
            let pos = mouse::position(ctx);
            match plant_type {
                PlantType::Sunflower => {
                    let preview_sunflower_mesh = graphics::Mesh::new_circle(
                        ctx,
                        graphics::DrawMode::fill(),
                        pos,
                        CELL_SIZE / 2.0 - 5.0,
                        32.0,
                        Color::from_rgba(255, 255, 0, 128),
                    )?;
                    canvas.draw(&preview_sunflower_mesh, DrawParam::default());
                }
                PlantType::Peashooter => {
                    let preview_peashooter_mesh = graphics::Mesh::new_rectangle(
                        ctx,
                        graphics::DrawMode::fill(),
                        Rect::new(pos.x - 20.0, pos.y - 30.0, 40.0, 60.0),
                        Color::from_rgba(0, 128, 0, 128),
                    )?;
                    canvas.draw(&preview_peashooter_mesh, DrawParam::default());
                }
            }
        }

        canvas.finish(ctx)
    }

    fn mouse_button_down_event(
        &mut self,
        ctx: &mut Context,
        button: mouse::MouseButton,
        x: f32,
        y: f32,
    ) -> GameResult {
        if self.game_over {
            return Ok(());
        }

        if button != mouse::MouseButton::Left {
            return Ok(());
        }

        // 处理工具栏点击（选择植物）
        if y < TOOLBAR_HEIGHT {
            // 向日葵按钮（100x60在(100,20)）
            if x > 100.0 && x < 180.0 && y > 20.0 && y < 80.0 {
                self.selected_plant = Some(PlantType::Sunflower);
            }
            // 豌豆射手按钮（100x60在(200,20)）
            else if x > 200.0 && x < 280.0 && y > 20.0 && y < 80.0 {
                self.selected_plant = Some(PlantType::Peashooter);
            }
            // 取消选择按钮（100x60在(300,20)）
            else if x > 300.0 && x < 380.0 && y > 20.0 && y < 80.0 {
                self.selected_plant = None;
            }
            return Ok(());
        }

        // 处理阳光收集
        for sun in self.suns.iter_mut() {
            if!sun.is_collected && (sun.position.x - x).abs() < 20.0 && (sun.position.y - y).abs() < 20.0 {
                sun.is_collected = true;
                self.sun += 25;
            }
        }
        self.suns.retain(|s|!s.is_collected);

        // 处理战斗区域点击（放置植物）
        if let Some(cell) = MyGame::screen_to_cell(x, y) {
            // 检查是否已存在植物
            if self.plants.iter().any(|p| p.cell == cell) {
                return Ok(());
            }

            // 检查阳光和选中植物
            if let Some(plant_type) = self.selected_plant {
                let cost = MyGame::get_plant_cost(plant_type);
                if self.sun >= cost {
                    self.sun -= cost;
                    self.plants.push(Plant {
                        cell,
                        plant_type,
                        health: 100,
                        last_sun_time: 0,
                        last_shoot_time: 0,
                    });
                }
            }
        }

        Ok(())
    }
}

// 辅助方法：绘制植物选择按钮
impl MyGame {
    fn draw_plant_selector(
        &self,
        canvas: &mut Canvas,
        ctx: &mut Context,
        plant_type: PlantType,
        x: f32,
        y: f32,
    ) -> GameResult {
        let button_rect = Rect::new(x, y, 80.0, 60.0);
        let button_mesh = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::fill(),
            button_rect,
            Color::WHITE,
        )?;
        canvas.draw(&button_mesh, DrawParam::default());

        match plant_type {
            PlantType::Sunflower => {
                let sunflower_button_mesh = graphics::Mesh::new_circle(
                    ctx,
                    graphics::DrawMode::fill(),
                    Vec2::new(x + 40.0, y + 30.0),
                    25.0,
                    32.0,
                    Color::YELLOW,
                )?;
                canvas.draw(&sunflower_button_mesh, DrawParam::default());
            }
            PlantType::Peashooter => {
                let peashooter_button_mesh = graphics::Mesh::new_rectangle(
                    ctx,
                    graphics::DrawMode::fill(),
                    Rect::new(x + 20.0, y + 10.0, 40.0, 50.0),
                    Color::GREEN,
                )?;
                canvas.draw(&peashooter_button_mesh, DrawParam::default());
            }
        }

        // 显示阳光消耗
        let cost = MyGame::get_plant_cost(plant_type);
        let cost_text = graphics::Text::new(format!("{}", cost));
        canvas.draw(
            &cost_text,
            DrawParam::default()
               .dest(Vec2::new(x + 60.0, y + 50.0))
               .color(Color::from_rgb(169, 169, 169)),
        );

        Ok(())
    }

    // 辅助方法：绘制取消选择按钮
    fn draw_cancel_button(
        &self,
        canvas: &mut Canvas,
        ctx: &mut Context,
        x: f32,
        y: f32,
    ) -> GameResult {
        let button_rect = Rect::new(x, y, 80.0, 60.0);
        let button_mesh = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::fill(),
            button_rect,
            Color::RED,
        )?;
        canvas.draw(&button_mesh, DrawParam::default());

        let cancel_text = graphics::Text::new("Cancel");
        canvas.draw(
            &cancel_text,
            DrawParam::default()
               .dest(Vec2::new(x + 20.0, y + 25.0))
               .color(Color::WHITE),
        );

        Ok(())
    }
}

fn main() {
    let (mut ctx, event_loop) = ContextBuilder::new("pvz", "豆包")
       .window_mode(ggez::conf::WindowMode::default().dimensions(WINDOW_WIDTH, WINDOW_HEIGHT))
       .build()
       .expect("初始化失败");

    let my_game = MyGame::new(&mut ctx);
    event::run(ctx, event_loop, my_game);
}