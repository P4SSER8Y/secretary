<script setup lang="ts">
import { computed, getCurrentInstance, onMounted, Ref, ref, watch } from 'vue';
import Waterfall from './pages/Waterfall.vue';
import Gallery from './pages/Gallery.vue';
import { raven } from './raven';
import { ApiListParams, MemeList, Meta, PageType, SortKey, TokenPayload } from './lib/struct';
import { debounce } from 'lodash';
import { useConfigStore } from './lib/configStore';
import { storeToRefs } from 'pinia';
import Upload from './pages/Upload.vue';
import FullScreenPreview from './pages/FullScreenPreview.vue';
import TagCloud from './pages/TagCloud.vue';

const HODOR_ENTRY = import.meta.env.VITE_HODOR_ENTRY;
const config = useConfigStore();
const { page, waterfall_pagnition } = storeToRefs(config);
const api = getCurrentInstance()?.appContext.config.globalProperties.$api;
let token: Ref<string | null> = ref(null);
let payload: Ref<TokenPayload | null> = computed(() => token.value && JSON.parse(atob(token.value.split('.')[1])));
let expire: Ref<number> = ref(0);
let display_expire = computed(() => {
    const minutes = Math.floor(expire.value / 60000);
    const seconds = Math.floor((expire.value % 60000) / 1000);
    const padZero = (num: number) => (num < 10 ? `0${num}` : `${num}`);
    return `${padZero(minutes)}:${padZero(seconds)}`;
});
let name: Ref<string | null> = ref(null);
let data: Ref<MemeList | null> = ref(null);
let filter: Ref<string> = ref('');
let is_asc: Ref<boolean> = ref(false);
let sort = ref(SortKey.timestamp);
let is_randomized = ref(true);
let single_preview: Ref<Meta | null> = ref(null);
let is_tag_cloud_shown = ref(false);

const update = debounce(async function update() {
    if (!token.value) {
        data.value = null;
        return;
    }
    let params: ApiListParams = {};
    if (filter.value.length > 0) {
        params.filter = filter.value;
    }
    if (!is_randomized.value) {
        params.asc = is_asc.value;
        params.sort = sort.value;
    }
    try {
        let res = await api?.get('list', {
            headers: { token: token.value },
            params: params,
        });
        if (is_randomized.value) {
            let temp = res?.data as MemeList | null;
            if (temp) {
                for (let i = temp.meta.length - 1; i > 0; i--) {
                    const j = Math.floor(Math.random() * (i + 1));
                    [temp.meta[i], temp.meta[j]] = [temp.meta[j], temp.meta[i]];
                }
            }
            data.value = temp;
        } else {
            data.value = res?.data as MemeList | null;
        }
    } catch {
        data.value = null;
    }
}, 500);

const force_update_cost: Ref<string | null> = ref(null);
const force_update = debounce(function () {
    if (token.value) {
        force_update_cost.value = "0.00";
        let now = new Date().getTime();
        let timer_id = setInterval(() => {
            let cost = Math.floor((new Date().getTime() - now)) / 1000;
            force_update_cost.value = cost.toFixed(2);
        }, 33);
        api?.get('update')
            .then(() => {
                update();
            })
            .finally(() => {
                clearTimeout(timer_id);
                force_update_cost.value = null;
            });
    }
}, 1000);

async function logout() {
    filter.value = '';
    token.value = null;
    data.value = null;
}

function show(meta: Meta) {
    single_preview.value = meta;
}

function uploaded(meta: Meta) {
    update();
    console.log(`uploaded ${meta.uuid} with tags: ${meta.tags}`);
    show(meta);
}

function check_token() {
    if (token.value && payload.value) {
        let now = new Date().getTime();
        expire.value = payload.value.exp * 1000 - now;
        if (expire.value > 0) {
            setTimeout(check_token, 1000);
        } else {
            token.value = null;
        }
    }
}

function commit_filter(data: string) {
    filter.value = data;
}

function delete_item(uuid: string) {
    if (data.value) {
        data.value.meta = data.value.meta.filter((item) => item.uuid !== uuid);
    }
}

watch(token, (newVal) => {
    if (newVal) {
        const payload = JSON.parse(atob(newVal.split('.')[1]));
        name.value = payload.n;
        document.cookie = `token=${newVal};maxAge=-1`;
        update();
        check_token();
    } else {
        document.cookie = `token=`;
        name.value = null;
    }
});

watch([filter, is_asc, sort, is_randomized], update);

onMounted(() => {
    token.value =
        document.cookie
            .split(';')
            .find((c) => c.trim().startsWith('token='))
            ?.split('=')[1] ?? null;
});
</script>

<template>
    <div class="fixed navbar bg-base-100 z-50 opacity-0 hover:opacity-90 max-md:opacity-90 duration-300 ease-in-out rounded-3xl">
        <div class="flex-1 min-w-0 w-full">
            <label v-if="token" class="w-full flex items-center gap-2">
                <div class="w-full flex-1 indicator">
                    <input type="text" placeholder="" class="input input-ghost w-full" v-model="filter" />
                    <span class="indicator-item badge font-mono">{{ data?.meta.length ?? 0 }}</span>
                </div>
                <div class="join">
                    <button class="join-item btn btn-sm btn-ghost" @click="is_tag_cloud_shown = true">𐄳</button>
                    <button class="join-item btn btn-sm btn-ghost" @click="filter = ''">X</button>
                </div>
            </label>
        </div>
        <div v-if="token" class="dropdown dropdown-end">
            <div tabindex="0" role="button" class="btn btn-ghost">
                <div class="w-2">↓</div>
            </div>
            <ul tabindex="0" class="menu menu-sm dropdown-content bg-base-100 rounded-box z-1 mt-3 w-80 p-2 shadow">
                <li>
                    <span>user</span>
                    <ul>
                        <li>
                            <a>{{ name }}</a>
                        </li>
                        <li>
                            <a @click="logout">logout in <span class="font-mono">{{ display_expire }}</span></a>
                        </li>
                        <li>
                            <a @click="force_update">
                                update
                                <span v-if="force_update_cost" class="font-mono"> {{ force_update_cost }}s </span>
                            </a>
                        </li>
                    </ul>
                </li>
                <li>
                    <span>config</span>
                    <ul>
                        <li>
                            <span>general</span>
                            <ul>
                                <li>
                                    <div class="flex"><input type="checkbox" class="toggle" v-model="is_randomized" />
                                        random </div>
                                </li>
                                <li v-if="!is_randomized">
                                    <div class="flex" @click="() => (is_asc = !is_asc)">
                                        <span>order</span>
                                        <span>{{ is_asc ? '↑' : '↓' }}</span>
                                    </div>
                                </li>
                                <li v-if="!is_randomized">
                                    <details>
                                        <summary>sort by {{ sort }}</summary>
                                        <ul>
                                            <li>
                                                <span @click="() => (sort = SortKey.timestamp)">
                                                    {{ SortKey.timestamp }}
                                                </span>
                                            </li>
                                            <li>
                                                <span @click="() => (sort = SortKey.uuid)">
                                                    {{ SortKey.uuid }}
                                                </span>
                                            </li>
                                        </ul>
                                    </details>
                                </li>
                            </ul>
                        </li>
                        <li>
                            <span>page</span>
                            <ul>
                                <li>
                                    <div @click="() => (page = PageType.Waterfall)">
                                        <input type="radio" class="radio" :checked="page == PageType.Waterfall"
                                            @click="() => (is_asc = !is_asc)" />
                                        <span>waterfall</span>
                                    </div>
                                </li>
                                <li>
                                    <div class="flex" @click="() => (page = PageType.Gallery)">
                                        <input type="radio" class="radio" :checked="page == PageType.Gallery" />
                                        <span>gallery</span>
                                    </div>
                                </li>
                            </ul>
                        </li>
                    </ul>
                </li>
                <li v-if="page == PageType.Waterfall">
                    <span>range</span>
                    <ul>
                        <li>
                            <div>
                                <input type="range" class="range range-xs" min="10" max="50" step="5"
                                    v-model.number="waterfall_pagnition" />
                                <div>{{ waterfall_pagnition }}</div>
                            </div>
                        </li>
                    </ul>
                </li>
            </ul>
        </div>
        <div v-else class="btn btn-ghost text-2xl" @click="
            () => {
                raven(HODOR_ENTRY).then((res) => (token = res));
            }
        ">
            ⛭
        </div>
    </div>
    <Waterfall v-if="token && page == PageType.Waterfall" :data="data" @show="show"> </Waterfall>
    <Gallery v-else-if="token && page == PageType.Gallery" :data="data"></Gallery>
    <Upload v-if="token && !single_preview" @done="uploaded"></Upload>
    <FullScreenPreview v-if="token && single_preview" :meta="single_preview" @end="() => (single_preview = null)"
        @deleted="(uuid) => delete_item(uuid)">
    </FullScreenPreview>
    <TagCloud v-if="is_tag_cloud_shown" :data="data" @commit="commit_filter" @quit="is_tag_cloud_shown = false">
    </TagCloud>
</template>

<style scoped lang="postcss">
@reference "tailwindcss";
</style>
