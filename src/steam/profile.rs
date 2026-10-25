use eframe::emath::{pos2, vec2, Pos2, Rect, Vec2};
use serde::{Deserialize, Serialize};

/// Approximate Steam profile page metrics. Page space has x = 0 on the page's
/// centre line (where Steam centres the background) and y = 0 at the top.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileGeometry {
    pub viewport_width: f32,
    pub content_width: f32,
    pub header_top: f32,
    pub header_height: f32,
    pub header_padding: f32,
    pub avatar_size: f32,
    pub body_gap: f32,
    pub left_column_x: f32,
    pub left_column_width: f32,
    pub right_column_width: f32,
    pub column_gap: f32,
    pub showcase_header_height: f32,
    pub showcase_padding: f32,
    pub showcase_spacing: f32,
    pub right_column_height: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct ProfileLayout {
    pub page: Rect,
    pub content: Rect,
    pub header: Rect,
    pub avatar: Rect,
    pub name_pos: Pos2,
    pub level_pos: Pos2,
    pub right_col: Rect,
    pub showcase: Rect,
    pub showcase_header: Rect,
    /// Page-space position of composition (0, 0).
    pub comp_origin: Pos2,
    pub activity: Rect,
}

impl ProfileGeometry {
    pub fn arrange(&self, comp: Vec2) -> ProfileLayout {
        let half = self.viewport_width / 2.0;
        let cl = -self.content_width / 2.0;
        let pad = self.header_padding;

        let header = Rect::from_min_size(pos2(cl, self.header_top), vec2(self.content_width, self.header_height));
        let avatar = Rect::from_min_size(header.min + vec2(pad, pad), Vec2::splat(self.avatar_size));
        let left_x = cl + self.left_column_x;
        let right_x = left_x + self.left_column_width + self.column_gap;

        let body_top = header.max.y + self.body_gap;
        let showcase_header = Rect::from_min_size(
            pos2(left_x, body_top),
            vec2(self.left_column_width, self.showcase_header_height),
        );
        let comp_origin = pos2(
            (left_x + (self.left_column_width - comp.x) / 2.0).round(),
            showcase_header.max.y + self.showcase_padding,
        );
        let showcase = Rect::from_min_max(
            showcase_header.min,
            pos2(left_x + self.left_column_width, comp_origin.y + comp.y + self.showcase_padding),
        );
        let activity = Rect::from_min_size(
            pos2(left_x, showcase.max.y + self.showcase_spacing),
            vec2(self.left_column_width, 236.0),
        );
        let right_col = Rect::from_min_size(
            pos2(right_x, body_top),
            vec2(self.right_column_width, self.right_column_height),
        );
        let bottom = activity.max.y.max(right_col.max.y) + 72.0;
        let page = Rect::from_min_max(pos2(-half, 0.0), pos2(half, bottom));
        let content = Rect::from_min_max(pos2(cl, 0.0), pos2(-cl, bottom));

        ProfileLayout {
            page,
            content,
            header,
            avatar,
            name_pos: pos2(avatar.max.x + 24.0, avatar.min.y + 10.0),
            level_pos: pos2(right_x, avatar.min.y + 10.0),
            right_col,
            showcase,
            showcase_header,
            comp_origin,
            activity,
        }
    }
}
