<script>
    import github from "images/github.png";
    import twitter from "images/twitter.png";
</script>

<footer class="footer">
    <div>
        <p>
            <span
                ><a href="https://twitter.com/blockchain44" target="_blank"
                    ><img src={twitter} alt="twitter" /></a
                ></span
            >
            <span
                ><a href="https://github.com/BlockchainEtSociete/codelab-18-mai-2022" target="_blank"
                    ><img src={github} alt="github" /></a
                ></span
            >
        </p>
    </div>
</footer>

<style>
    .footer {
        width: 100%;
        border-top: 1px solid lightgray;
        margin-top: 5rem;
        color: #575757;
        float: right;
    }

    p {
        width: fit-content;
        margin-left: auto;
        margin-right: 0;
    }

    span {
        padding-left: 5px;
    }
</style>
