<script setup lang="ts">
import { computed, getCurrentInstance, onMounted, provide, Ref, ref, watch } from 'vue';
import Waterfall from './pages/Waterfall.vue';
import Gallery from './pages/Gallery.vue';
import { ravenOpen, ravenWait, ravenUrl } from './raven';
import { ApiListParams, MemeList, Meta, PageType, SortKey, TokenPayload } from './lib/struct';
import { debounce } from 'lodash';
import { useConfigStore } from './lib/configStore';
import { storeToRefs } from 'pinia';
import Upload from './pages/Upload.vue';
import FullScreenPreview from './pages/FullScreenPreview.vue';
import TagCloud from './pages/TagCloud.vue';
import ReencryptDialog from './pages/ReencryptDialog.vue';

const config = useConfigStore();
const { page, waterfall_pagnition } = storeToRefs(config);
const api = getCurrentInstance()?.appContext.config.globalProperties.$api;
let token: Ref<string | null> = ref(null);

// 从后端拉取认证（gate）入口路径，由 meme.jwt_gate 配置决定
async function gateLogin() {
    // window.open 必须在用户手势的同步调用栈内执行，否则 Safari 会拦截弹窗。
    // 先同步打开同源占位窗口，再在后台拉取 gate 路径后导航到真实认证页。
    let win: Window;
    try {
        win = ravenOpen();
    } catch (e) {
        console.error('open gate window failed', e);
        return;
    }
    try {
        let gate = '/gate/';
        if (api) {
            const resp = await api.get('gate');
            if (resp.data?.gate) gate = resp.data.gate;
        }
        win.location.href = ravenUrl(gate).href;
    } catch (e) {
        console.error('fetch gate path failed, fallback', e);
    }
    token.value = await ravenWait(win);
}
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
let reencrypt_shown = ref(false);
let password: Ref<string> = ref('');
provide('password', password);
// 上次真正应用过的密码，用于跳过"点击输入框但没改密码"这类无意义刷新。
let committed_password = '';

// 拉取列表（GET /list）。服务端会按当前密码惰性判断是否需要同步，
// 同一密码只同步一次，因此登录/换密码都能在第一次拉取时拿到新数据。
async function fetch_list() {
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
}

const update = debounce(fetch_list, 500);

const force_update_cost: Ref<string | null> = ref(null);
let syncing = false;
let sync_pending = false;
let sync_cost_timer: ReturnType<typeof setInterval> | undefined;

// 全量重新同步（GET /update）后再拉列表，仅由手动 "update" 菜单触发。
// 合并并发调用：同一时刻至多一个同步在跑，期间新来的请求只记一个 pending，
// 等当前同步结束后用最新密码补跑一次，避免重复同步与重复倒计时。
async function do_sync() {
    if (!token.value) return;
    if (syncing) {
        sync_pending = true;
        return;
    }
    syncing = true;
    data.value = null;
    force_update_cost.value = "0.00";
    let start = new Date().getTime();
    sync_cost_timer = setInterval(() => {
        force_update_cost.value = ((new Date().getTime() - start) / 1000).toFixed(2);
    }, 100);
    try {
        await api?.get('update');
        await fetch_list();
    } catch (e) {
        console.error('sync failed', e);
    } finally {
        clearInterval(sync_cost_timer);
        sync_cost_timer = undefined;
        force_update_cost.value = null;
        syncing = false;
        if (sync_pending) {
            sync_pending = false;
            void do_sync();
        }
    }
}
const force_update = debounce(() => void do_sync(), 1000);

async function logout() {
    filter.value = '';
    token.value = null;
    data.value = null;
    password.value = '';
    committed_password = '';
    sessionStorage.removeItem('password');
}

function show(meta: Meta) {
    single_preview.value = meta;
}

function uploaded(meta: Meta) {
    if (page.value == PageType.Waterfall) {
        is_randomized.value = false;
        sort.value = SortKey.timestamp;
        is_asc.value = false;
    }
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
        // 登录即全量同步一次，保证看到的是最新数据，无需手动 update。
        // 后续 `list` 按密钥惰性同步，同一密码不再重复打 S3。
        void do_sync();
        check_token();
    } else {
        document.cookie = `token=`;
        name.value = null;
    }
});

watch([filter, is_asc, sort, is_randomized], update);

// 密码即时写入 sessionStorage，让后续所有请求带上当前密码。
watch(password, (v) => {
    if (v) {
        sessionStorage.setItem('password', v);
    } else {
        sessionStorage.removeItem('password');
    }
});

// 密码稳定（输入停顿）后自动应用：只拉列表，由服务端按密钥惰性同步；
// 密码未变则跳过。去掉了原来的 blur/空值强制 update。
const apply_password = debounce(() => {
    if (password.value !== committed_password) {
        committed_password = password.value;
        update();
    }
}, 800);
watch(password, apply_password);

// 回车立即应用密码。
function commit_password() {
    apply_password.cancel();
    if (password.value !== committed_password) {
        committed_password = password.value;
        update();
    }
}

onMounted(() => {
    token.value =
        document.cookie
            .split(';')
            .find((c) => c.trim().startsWith('token='))
            ?.split('=')[1] ?? null;
    sessionStorage.removeItem('password');
    password.value = '';
    committed_password = '';
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
                        <li>
                            <a @click="reencrypt_shown = true">re-encrypt</a>
                        </li>
                        <li>
                            <div class="px-2 py-1">
                                <input
                                    type="password"
                                    placeholder="vault password"
                                    class="input input-ghost input-xs w-full"
                                    v-model="password"
                                    @keyup.enter="commit_password()"
                                />
                            </div>
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
        <div v-else class="btn btn-ghost text-2xl" @click="gateLogin">
            ⛭
        </div>
    </div>
    <Waterfall v-if="token && page == PageType.Waterfall" :data="data" @show="show"> </Waterfall>
    <Gallery v-else-if="token && page == PageType.Gallery" :data="data"></Gallery>
    <Upload v-if="token" @done="uploaded" @begin="() => (single_preview = null)"></Upload>
    <FullScreenPreview v-if="token && single_preview" :meta="single_preview" @end="() => (single_preview = null)"
        @deleted="(uuid) => delete_item(uuid)"
        @tags-updated="() => update()">
    </FullScreenPreview>
    <TagCloud v-if="is_tag_cloud_shown" :data="data" @commit="commit_filter" @quit="is_tag_cloud_shown = false">
    </TagCloud>
    <ReencryptDialog v-if="reencrypt_shown" :filter="filter" :password="password"
        @quit="reencrypt_shown = false"
        @done="(newPwd: string) => { password = newPwd; reencrypt_shown = false; }">
    </ReencryptDialog>
</template>

<style scoped lang="postcss">
@reference "tailwindcss";
</style>
