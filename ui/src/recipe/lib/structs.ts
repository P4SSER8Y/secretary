export interface Ingredient {
    name: string
    amount?: number
    unit?: string
    hint?: string
    optional: boolean
}

export interface ScaledIngredient {
    name: string
    amount?: number
    unit?: string
    hint?: string
    optional: boolean
    used_in: string[]
}

export interface RecipeMeta {
    id: string
    name: string
    category: string
    cover_image?: string
    prep_time: string
    cook_time: string
    servings: number
    difficulty: string
    adjustable: boolean
    ingredients: Ingredient[]
    tags: string[]
    body: string
}

export interface RecipeSummary {
    id: string
    name: string
    category: string
    cover_image?: string
    cook_time: string
    difficulty: string
    tags: string[]
    servings: number
    adjustable: boolean
}

export interface MenuSummary {
    filename: string
    name: string
    date: string
    diners: number
    dish_count: number
}

export interface MenuRecipe {
    id: string
    portions: number
}

export interface MenuState {
    filename: string
    name: string
    date: string
    diners: number
    menu_recipes: MenuRecipe[]
    ingredients: ScaledIngredient[]
    recipes: RecipeMeta[]
}

export interface ApiResponse<T> {
    ok: boolean
    message?: string
    data?: T
}
