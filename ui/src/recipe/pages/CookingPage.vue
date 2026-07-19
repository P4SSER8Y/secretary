<script setup lang="ts">
import { ref, computed, getCurrentInstance } from 'vue'
import type { AxiosInstance } from 'axios'
import { useRecipeStore } from '../lib/store'
import type { RecipeMeta } from '../lib/structs'

const api = getCurrentInstance()?.appContext.config.globalProperties.$api as AxiosInstance
const store = useRecipeStore()

const selectedDishIndex = ref(0)
const recipeDetail = ref<RecipeMeta | null>(null)
const loading = ref(false)

// Combined dish list: recipes + custom dishes
interface DishTab {
    name: string
    isCustom: boolean
    portions?: number
}

const dishList = computed<DishTab[]>(() => {
    const recipes: DishTab[] = (store.menu?.recipes || []).map(r => ({
        name: r.name,
        isCustom: false,
    }))
    const customs: DishTab[] = (store.menu?.custom_dishes || []).map((cd, i) => ({
        name: `__custom__${i}`,
        isCustom: true,
        portions: cd.portions,
    }))
    return [...recipes, ...customs]
})

const selectedIsCustom = computed(() => dishList.value[selectedDishIndex.value]?.isCustom ?? false)
const currentCustomPortions = computed(() => {
    const d = dishList.value[selectedDishIndex.value]
    return d?.isCustom ? d.portions : null
})

function scaleIng(ing: import('../lib/structs').Ingredient, ratio: number): import('../lib/structs').Ingredient {
    return {
        ...ing,
        amount: ing.amount != null ? Math.round(ing.amount * ratio * 100) / 100 : undefined,
        sub_ingredients: ing.sub_ingredients?.map(sub => scaleIng(sub, ratio)),
    }
}

const scaledIngredients = computed(() => {
    if (!recipeDetail.value || !store.menu) return recipeDetail.value?.ingredients || []
    const mr = store.menu.menu_recipes.find(mr => mr.name === recipeDetail.value!.name)
    if (!mr) return recipeDetail.value.ingredients
    const ratio = mr.portions / recipeDetail.value.servings
    if (ratio === 1) return recipeDetail.value.ingredients
    return recipeDetail.value.ingredients.map(ing => scaleIng(ing, ratio))
})

const currentPortions = computed(() => {
    if (!recipeDetail.value || !store.menu) return null
    const mr = store.menu.menu_recipes.find(mr => mr.name === recipeDetail.value!.name)
    return mr ? mr.portions : null
})

async function selectDish(index: number) {
    selectedDishIndex.value = index
    const dish = dishList.value[index]
    if (!dish) return
    if (dish.isCustom) {
        recipeDetail.value = null
        return
    }
    loading.value = true
    recipeDetail.value = await store.loadRecipeDetail(api, dish.name)
    loading.value = false
}

// Auto-select first dish
if (dishList.value.length > 0 && !recipeDetail.value) {
    selectDish(0)
}

function onAdjust(delta: number) {
    if (!recipeDetail.value) return
    store.adjustPortion(api, recipeDetail.value.name, delta)
}

function onAdjustCustom(delta: number) {
    const d = dishList.value[selectedDishIndex.value]
    if (!d?.isCustom || d.portions == null) return
    const newPortion = Math.max(0.5, Math.round((d.portions + delta) * 2) / 2)
    // Extract the custom dish index from the name
    const idx = parseInt(d.name.replace('__custom__', ''))
    store.adjustCustomPortion(api, idx, newPortion)
}

function goBack() {
    store.screen = 'menu'
}
</script>

<template>
    <div class="min-h-dvh bg-base-100 text-base-content">
        <!-- Top bar -->
        <div class="navbar bg-base-200 shadow-sm px-2 gap-1">
            <button class="btn btn-sm btn-ghost" @click="goBack">← 菜单</button>
            <h1 class="text-base font-bold truncate">{{ store.menu?.name }} · 做菜</h1>
        </div>

        <!-- Dish tabs -->
        <div class="flex gap-1 px-3 py-2 overflow-x-auto">
            <button
                v-for="(dish, i) in dishList"
                :key="dish.name"
                class="btn btn-sm whitespace-nowrap"
                :class="selectedDishIndex === i ? 'btn-primary' : 'btn-ghost'"
                @click="selectDish(i)"
            >
                {{ dish.name }}
            </button>
        </div>

        <!-- Custom dish: simple title-only view -->
        <div v-if="selectedIsCustom" class="px-4 pb-24">
            <div class="text-center py-20">
                <div class="text-6xl mb-6">🍽️</div>
                <h1 class="text-2xl font-bold mb-2">{{ dishList[selectedDishIndex]?.name }}</h1>
                <p class="text-base text-base-content/50 mb-4">自定义菜品（无详细步骤）</p>
                <!-- Portion control -->
                <div v-if="currentCustomPortions != null" class="flex items-center justify-center gap-2 p-2 rounded bg-base-200 w-fit mx-auto">
                    <span class="text-sm">份量</span>
                    <button class="btn btn-xs btn-ghost w-6 h-6" @click="onAdjustCustom(-0.5)">−</button>
                    <span class="badge badge-sm">{{ currentCustomPortions }}份</span>
                    <button class="btn btn-xs btn-ghost w-6 h-6" @click="onAdjustCustom(0.5)">+</button>
                </div>
            </div>
        </div>

        <!-- Recipe content -->
        <div v-else-if="loading" class="flex justify-center py-20">
            <span class="loading loading-spinner loading-lg"></span>
        </div>

        <div v-else-if="recipeDetail" class="px-4 pb-24">
            <!-- Cover image -->
            <img
                v-if="recipeDetail.cover_image"
                :src="`api/image/${encodeURIComponent(recipeDetail.name)}`"
                :alt="recipeDetail.name"
                class="w-full aspect-[5/3] object-cover rounded-lg mt-3"
            />

            <!-- Meta -->
            <div class="flex gap-3 text-base text-base-content/60 mt-3 mb-4">
                <span>⏱ {{ recipeDetail.cook_time }}</span>
                <span>📊 {{ recipeDetail.difficulty }}</span>
                <span>👥 {{ recipeDetail.servings }}人份</span>
            </div>

            <!-- Portion control -->
            <div v-if="recipeDetail.adjustable" class="flex items-center gap-2 mb-4 p-2 rounded bg-base-200">
                <span class="text-sm">份量</span>
                <button class="btn btn-xs btn-ghost w-6 h-6" @click="onAdjust(-0.5)">−</button>
                <span class="badge badge-sm">{{ currentPortions ?? recipeDetail.servings }}份</span>
                <button class="btn btn-xs btn-ghost w-6 h-6" @click="onAdjust(0.5)">+</button>
                <span class="text-xs text-base-content/50">(原{{ recipeDetail.servings }}人份)</span>
            </div>

            <!-- Ingredients -->
            <div class="mb-4">
                <h3 class="text-base font-semibold mb-2">
                    食材
                    <span v-if="currentPortions && currentPortions !== recipeDetail.servings" class="text-base-content/50 font-normal text-sm ml-1">
                        ({{ recipeDetail.servings }}人份 → {{ currentPortions }}份)
                    </span>
                </h3>
                <div class="text-base space-y-1">
                    <template v-for="ing in scaledIngredients" :key="ing.name">
                        <!-- Compound ingredient group -->
                        <div v-if="ing.sub_ingredients && ing.sub_ingredients.length > 0"
                            class="rounded bg-base-200 overflow-hidden"
                        >
                            <div class="flex items-center gap-2 px-3 py-1.5 bg-base-300/50">
                                <span class="text-sm font-bold">📦 {{ ing.name }}</span>
                                <span v-if="ing.hint" class="text-xs text-base-content/50">{{ ing.hint }}</span>
                            </div>
                            <div class="px-2 py-1 space-y-0.5">
                                <div v-for="(sub, j) in ing.sub_ingredients" :key="j"
                                    class="flex justify-between py-0.5 pl-4"
                                >
                                    <span class="text-sm">{{ sub.name }}</span>
                                    <span class="text-sm text-base-content/60">
                                        {{ sub.amount ? `${sub.amount}${sub.unit || ''}` : sub.hint || '' }}
                                    </span>
                                </div>
                            </div>
                        </div>
                        <!-- Flat ingredient -->
                        <div v-else class="flex justify-between">
                            <span>{{ ing.name }}</span>
                            <span class="text-base-content/60">
                                {{ ing.amount ? `${ing.amount}${ing.unit || ''}` : ing.hint || '' }}
                            </span>
                        </div>
                    </template>
                </div>
            </div>

            <div class="divider my-2"></div>

            <!-- Markdown body rendered as HTML-ish -->
            <div class="prose prose-sm max-w-none" v-html="recipeDetail.body
                .replace(/^### (.+)$/gm, '<h3 class=\'text-base font-bold mt-4 mb-2\'>$1</h3>')
                .replace(/^## (.+)$/gm, '<h2 class=\'text-lg font-bold mt-4 mb-2\'>$1</h2>')
                .replace(/^# (.+)$/gm, '<h1 class=\'text-xl font-bold mt-4 mb-2\'>$1</h1>')
                .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
                .replace(/^> (.+)$/gm, '<blockquote class=\'border-l-2 border-accent pl-3 my-2 text-base-content/60\'>$1</blockquote>')
                .replace(/^- (.+)$/gm, '<li class=\'ml-4\'>$1</li>')
                .replace(/^(\d+)\. (.+)$/gm, '<div class=\'flex gap-2 my-1\'><span class=\'font-bold text-accent\'>$1.</span><span>$2</span></div>')
                .replace(/\n\n/g, '<br/>')
            "></div>
        </div>
    </div>
</template>

<style scoped lang="postcss">
@reference "tailwindcss";
</style>
