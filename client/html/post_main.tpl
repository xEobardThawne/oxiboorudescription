<div class='content-wrapper transparent post-view'>
    <aside class='sidebar'>
        <nav class='buttons'>
            <article class='previous-post'>
                <% if (ctx.prevPostId) { %>
                    <% if (ctx.editMode) { %>
                        <a rel='prev' href='<%= ctx.getPostEditUrl(ctx.prevPostId, ctx.parameters) %>'>
                    <% } else { %>
                        <a rel='prev' href='<%= ctx.getPostUrl(ctx.prevPostId, ctx.parameters) %>'>
                    <% } %>
                <% } else { %>
                    <a rel='prev' class='inactive'>
                <% } %>
                    <i class='fa fa-chevron-left'></i>
                    <span class='vim-nav-hint'>&lt; Previous post</span>
                </a>
            </article>
            <article class='next-post'>
                <% if (ctx.nextPostId) { %>
                    <% if (ctx.editMode) { %>
                        <a rel='next' href='<%= ctx.getPostEditUrl(ctx.nextPostId, ctx.parameters) %>'>
                    <% } else { %>
                        <a rel='next' href='<%= ctx.getPostUrl(ctx.nextPostId, ctx.parameters) %>'>
                    <% } %>
                <% } else { %>
                    <a rel='next' class='inactive'>
                <% } %>
                    <i class='fa fa-chevron-right'></i>
                    <span class='vim-nav-hint'>Next post &gt;</span>
                </a>
            </article>
            <% if (ctx.canEditPosts || ctx.canDeletePosts || ctx.canFeaturePosts) { %>
            <article class='edit-post'>
                <% if (ctx.editMode) { %>
                    <a href='<%= ctx.getPostUrl(ctx.post.id, ctx.parameters) %>'>
                        <i class='fa fa-reply'></i>
                        <span class='vim-nav-hint'>Back to view mode</span>
                    </a>
                <% } else { %>
                    <a href='<%= ctx.getPostEditUrl(ctx.post.id, ctx.parameters) %>'>
                    <i class='fa fa-pencil'></i>
                    <span class='vim-nav-hint'>Edit post</span>
                    </a>
                <% } %>
            </article>
            <% } %>
        </nav>

        <div class='sidebar-container'></div>
    </aside>

    <div class='content'>
        <div class='post-container'></div>
        
        <% if (ctx.editMode) { %>
            <h2>Description</h2>
            <%= ctx.makeTextarea({
                id: 'post-description',
                value: ctx.post.description,
            }) %>
        <% } else if (ctx.post.description != undefined && ctx.post.description != '') { %>
            <div class='description-container'>
                <details open>
                    <summary>Description</summary>
                    <%= ctx.makeMarkdown(ctx.post.description) %>
                </details>
            </div>
        <% } %>
        
        <div class='after-mobile-controls'>
            <% if (ctx.canCreateComments) { %>
                <h2>Add comment</h2>
                <div class='comment-form-container'></div>
            <% } %>

            <% if (ctx.canListComments) { %>
                <div class='comments-container'></div>
            <% } %>
        </div>
    </div>
</div>
