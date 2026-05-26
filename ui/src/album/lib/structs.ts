export interface ImageMeta {
    id: string;
    original_filename: string;
    mime_type: string;
    created_at: string;
    params: ConversionParams;
}

export interface ConversionParams {
    crop_mode: string | null;
    dither: boolean;
    rotate_cw: boolean;
    rotate_ccw: boolean;
    invert: boolean;
}

export interface DeviceState {
    ip: string | null;
    online: boolean;
    status: string | null;
    last_seen: string | null;
}

export interface UploadResponse {
    ok: boolean;
    id: string;
    meta: ImageMeta;
}

export interface SwitchResponse {
    ok: boolean;
    message: string;
}
